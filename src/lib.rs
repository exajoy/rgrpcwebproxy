use async_stream::try_stream;
use bytes::{BufMut, Bytes, BytesMut};
use futures_core::Stream;
use http::{HeaderMap, HeaderValue, Request, Response, Uri, header::CONTENT_TYPE, uri::Authority};
use http_body::Frame;
use http_body_util::{BodyExt, StreamBody};
use hyper::{body::Incoming, client::conn::http2};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use std::pin::Pin;
use std::str::FromStr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tower::BoxError;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub mod grpcweb;
pub mod metadata;
pub mod status;
pub mod util;

type DynStream = Pin<Box<dyn Stream<Item = Result<Frame<Bytes>, hyper::Error>> + Send>>;
type StreamResponse = Response<StreamBody<DynStream>>;

pub async fn forward<B>(
    req: Request<B>,
    authority: Authority,
) -> Result<
    Response<StreamBody<impl Stream<Item = Result<Frame<Bytes>, hyper::Error>> + Send>>,
    BoxError,
>
where
    // B: Body + Send + 'static,
    B: hyper::body::Body<Data = Bytes> + Send + 'static + Unpin,
    B::Error: Into<BoxError>,
{
    //[START] switch endpoint
    let (mut parts, req_body) = req.into_parts();
    parts
        .headers
        .insert(hyper::header::HOST, authority.as_str().parse()?);

    let path = parts.uri.path();
    let url = format!("http://{}{}", authority.as_ref(), path);

    parts.uri = Uri::from(url.parse::<Uri>().unwrap());

    // println!("Forward URL: {}", parts.uri);
    // println!("Authority: {}", authority);

    //[END] switch endpoint

    let stream = TcpStream::connect(authority.to_string()).await?;
    let io = TokioIo::new(stream);

    let exec = TokioExecutor::new();
    let (sender, conn): (
        http2::SendRequest<_>,
        http2::Connection<TokioIo<TcpStream>, _, TokioExecutor>,
    ) = http2::Builder::new(exec).handshake(io).await?;

    // Spawn a task to poll the connection, driving the HTTP state
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    if let Some(value) = parts.headers.get(CONTENT_TYPE)
        && value == "application/grpc"
    {
        return forward_grpc(sender, parts, req_body).await;
    }
    return forward_grpc_web(sender, parts, req_body).await;
}

async fn forward_grpc_web<B>(
    mut sender: http2::SendRequest<B>,
    parts: http::request::Parts,
    req_body: B,
) -> Result<StreamResponse, BoxError>
where
    // B: Body,
    // B: Body + Send + 'static,
    B: hyper::body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError>,
{
    //[START] Convert to new request
    let mut req = Request::from_parts(parts, req_body);

    req.headers_mut().insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/grpc"),
    );

    req.headers_mut().remove(hyper::header::CONTENT_LENGTH);

    //[END]Convert to new request

    let fut = sender.send_request(req);
    let res = fut.await.map_err(Into::<BoxError>::into).map(|res| {
        let (parts, body) = res.into_parts();
        let res = Response::from_parts(parts, body);
        res
    })?;

    // println!("Status: {}", res.status());
    // println!("Initial headers:");
    // for (key, value) in res.headers() {
    //     println!("  {}: {:?}", key, value);
    // }
    // Read body data
    let (parts, mut body) = res.into_parts();

    let forward_stream = try_stream! {
        while let Some(frame) = body.frame().await {
            let frame = frame?;
            if let Some(trailers) = frame.trailers_ref() {
                for (k, v) in trailers.iter() {
                    println!("  {}: {:?}", k, v);
                }
                let trailer_frame = make_trailers_frame(trailers);
                // print_bytes_as_hex(&trailer_frame);
                yield Frame::data(trailer_frame.clone());
            } else {
                if let Some(_data) = frame.data_ref() {
                    // println!("Data: {:?}", data);
                }
                yield frame;
            }
        }
    };
    let boxed_stream: DynStream = Box::pin(forward_stream);
    // Convert stream into a Hyper body (chunked automatically)
    let body = StreamBody::new(boxed_stream);
    let mut res = Response::from_parts(parts, body);
    res.headers_mut().insert(
        "content-type",
        "application/grpc-web+proto".parse().unwrap(),
    );
    return Ok(res);
}

async fn forward_grpc<B>(
    mut sender: http2::SendRequest<B>,
    parts: http::request::Parts,
    req_body: B,
) -> Result<StreamResponse, BoxError>
where
    // B: Body,
    // B: Body + Send + 'static,
    B: hyper::body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError>,
{
    let req = Request::from_parts(parts, req_body);
    let res = sender.send_request(req).await?;

    // convert type Incoming to StreamBody<DynStream>
    // this does not affect the logic
    let res = res.map(|body| incoming_to_stream_body(body));
    return Ok(res);
}

pub fn incoming_to_stream_body(mut body: Incoming) -> StreamBody<DynStream> {
    let forward_stream = try_stream! {
        while let Some(frame) = body.frame().await {
            let frame = frame?;
            yield frame;
        }
    };
    return StreamBody::new(Box::pin(forward_stream));
}

fn encode_trailers(trailers: &HeaderMap) -> Vec<u8> {
    trailers.iter().fold(Vec::new(), |mut acc, (key, value)| {
        acc.put_slice(key.as_ref());
        acc.push(b':');
        acc.put_slice(value.as_bytes());
        acc.put_slice(b"\r\n");
        acc
    })
}

const FRAME_HEADER_SIZE: usize = 5;
// 8th (MSB) bit of the 1st gRPC frame byte
// denotes an uncompressed trailer (as part of the body)
const GRPC_WEB_TRAILERS_BIT: u8 = 0b10000000;
fn make_trailers_frame(trailers: &HeaderMap) -> Bytes {
    let trailers = encode_trailers(trailers);
    let len = trailers.len();
    assert!(len <= u32::MAX as usize);

    let mut frame = BytesMut::with_capacity(len + FRAME_HEADER_SIZE);
    frame.put_u8(GRPC_WEB_TRAILERS_BIT);
    frame.put_u32(len as u32);
    frame.put_slice(&trailers);

    frame.freeze()
}

pub async fn start_proxy(
    //proxy_address: &str,
    listener: TcpListener,
    forward_authority: String,
    mut shutdown_rx: watch::Receiver<bool>,
    // mut ready_tx: tokio::sync::oneshot::Sender<()>,
) -> Result<(), BoxError> {
    let forward_authority = Authority::from_str(&forward_authority)?;
    // let listener = TcpListener::bind(proxy_address).await?;
    // ready_tx.send(())?;
    // ready_tx.send(()).ok();
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _)) => {
                        let io = TokioIo::new(stream);
                        let forward_authority = forward_authority.clone();
                        tokio::task::spawn(async move {

                            let forward_authority = forward_authority.clone();
                            let svc = tower::service_fn(move |req| {
                                let forward_authority = forward_authority.clone();
                                forward(req, forward_authority)
                            });
                            let svc = TowerToHyperService::new(svc);
                            if let Err(err) = AutoBuilder::new(TokioExecutor::new())
                                .serve_connection(io, svc)
                                .await
                            {
                                eprintln!("Error serving connection: {:?}", err);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to accept connection: {:?}", e);
                    }
                }
            }

             _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    println!("Proxy shutdown signal received");
                    break;
                }
            }
        }
        // let (stream, _) = listener.accept().await?;
        // let io = TokioIo::new(stream);
        // let forward_authority = forward_authority.clone();
        // tokio::task::spawn(async move {
        //     let forward_authority = forward_authority.clone();
        //     let svc = tower::service_fn(move |req| {
        //         let forward_authority = forward_authority.clone();
        //         forward(req, forward_authority)
        //     });
        //     let svc = TowerToHyperService::new(svc);
        //     if let Err(err) = AutoBuilder::new(TokioExecutor::new())
        //         .serve_connection(io, svc)
        //         .await
        //     {
        //         eprintln!("Error serving connection: {:?}", err);
        //     }
        // });
    }
    drop(listener);
    Ok(())
    // listener.match
}

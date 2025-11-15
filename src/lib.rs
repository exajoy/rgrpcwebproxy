use bytes::Bytes;
use futures_core::Stream;
use http::{Request, Response, Uri, header::CONTENT_TYPE, uri::Authority};
use http_body::Frame;
use http_body_util::StreamBody;
use hyper::client::conn::http2;
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use scopeguard::defer;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::time::Instant;
use tower::BoxError;

use crate::core::grpc_kind::GrpcKind;
use crate::telemetry::metrics::Metrics;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub mod command;
pub mod core;
pub mod telemetry;
pub mod trailers;

pub async fn forward<B>(
    req: Request<B>,
    authority: Authority,
    metrics: Arc<Metrics>,
) -> Result<
    Response<StreamBody<impl Stream<Item = Result<Frame<Bytes>, hyper::Error>> + Send>>,
    BoxError,
>
where
    B: hyper::body::Body<Data = Bytes> + Send + 'static + Unpin,
    B::Error: Into<BoxError>,
{
    //[START] switch endpoint
    let (mut parts, req_body) = req.into_parts();
    parts
        .headers
        .insert(hyper::header::HOST, authority.as_str().parse()?);

    let path = parts.uri.path().to_string();

    // Early exit for /metrics
    if path == "/metrics" {
        return Ok(metrics.render());
    }
    let start = Instant::now();
    defer!({
        let elapsed = start.elapsed().as_secs_f64();
        metrics
            .requests_total
            .with_label_values(&[&"POST", &path.as_str()])
            .inc();
        metrics
            .request_duration
            .with_label_values(&[&"POST", &path.as_str()])
            .observe(elapsed);
    });
    let url = format!("http://{}{}", authority.as_ref(), path);

    parts.uri = url.parse::<Uri>()?;

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
    let content_type = parts
        .headers
        .get(CONTENT_TYPE)
        .ok_or("Missing Content-Type header")?
        .clone();
    let req = Request::from_parts(parts, req_body);
    GrpcKind::from_content_type(&content_type)
        .ok_or("Unsupported Content-Type header")?
        .forward(sender, req)
        .await
}

pub async fn start_proxy(
    listener: TcpListener,
    forward_authority: String,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), BoxError> {
    let forward_authority = Authority::from_str(&forward_authority)?;
    let metrics = Arc::new(Metrics::new());
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _)) => {
                        let io = TokioIo::new(stream);
                        let metrics = metrics.clone();
                        let forward_authority = forward_authority.clone();
                        tokio::task::spawn(async move {

                            let forward_authority = forward_authority.clone();
                            let metrics = metrics.clone();
                            let svc = tower::service_fn(move |req| {
                                let forward_authority = forward_authority.clone();
                                let metrics = metrics.clone();
                                forward(req, forward_authority, metrics)
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
    }
    Ok(())
}

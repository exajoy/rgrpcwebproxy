use bytes::Bytes;
use clap::Parser;
use http::Uri;
use http::uri::Authority;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::client::conn::http2;
use hyper::server::conn::http1;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::service::TowerToHyperService;
use tokio::net::{TcpListener, TcpStream};
use tower::BoxError;
use tower::ServiceBuilder;

use crate::grpcweb::args::Args;
use crate::grpcweb::call::GrpcWebCall;
use crate::grpcweb::grpc_header::*;

pub mod grpcweb;
pub mod metadata;
pub mod status;
pub mod util;

async fn forward(req: Request<Incoming>) -> Result<Response<GrpcWebCall<Full<Bytes>>>, BoxError> {
    //[START] switch endpoint
    let (mut parts, res_body) = req.into_parts();
    let args = Args::parse();
    // let authority = format!("", args.forward_host, args.forward_port);
    let authority = format!("{}:{}", args.forward_host, args.forward_port);
    let authority = Authority::from_maybe_shared(authority).unwrap();
    // this will resplace

    let path = parts.uri.path();
    let url = format!("http://{}{}", authority.as_ref(), path);

    parts.uri = Uri::from(url.parse::<Uri>().unwrap());
    println!("Forward URL: {}", parts.uri);
    println!("Authority: {}", authority);

    //[END] switch endpoint

    // let authority = req
    //     .uri()
    //     .authority()
    //     .ok_or("Request URI has no authority")?
    //     .to_string();

    let stream = TcpStream::connect(authority).await?;
    let io = TokioIo::new(stream);

    let exec = TokioExecutor::new();
    let (mut sender, conn) = http2::Builder::new(exec).handshake(io).await?;

    // Spawn a task to poll the connection, driving the HTTP state
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });
    let res = sender.send_request(req).await?;
    let (parts, body) = res.into_parts();
    let body = body.collect().await?;

    let res = Response::from_parts(
        parts,
        GrpcWebCall::client_response(Full::<Bytes>::from(body.to_bytes())),
    );
    Ok(res)
}
#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let args = Args::parse();
    let address = format!("{}:{}", args.proxy_host, args.proxy_port);
    let listener = TcpListener::bind(address).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        tokio::task::spawn(async move {
            let svc = tower::service_fn(forward);
            // let svc = ServiceBuilder::new().layer_fn(GrpcHeader::new).service(svc);
            let svc = TowerToHyperService::new(svc);
            if let Err(err) = http1::Builder::new().serve_connection(io, svc).await {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

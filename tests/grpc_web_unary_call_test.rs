use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures_core::Stream;
use futures_util::StreamExt;
use http::{Request, uri::Authority};
use http_body::{Body, Frame};
use http_body_util::{Full, StreamBody};
use httptest::{
    Expectation, Server, all_of,
    matchers::{contains, request},
    responders::{json_encoded, status_code},
};
use hyper::{body::Incoming, service::Service};
use hyper_util::{client::legacy::Client, rt::TokioExecutor, service::TowerToHyperService};
use prost::Message;

use rgrpcwebproxy::{
    forward, incoming_to_stream_body,
    test_support::{
        greeter::hello_world::{HelloReply, HelloRequest},
        preparation::run_intergration,
        utils::{message_to_frame, parse_grpc_stream},
    },
};
use tower::BoxError;

#[tokio::test]
async fn test_grpc_web_unary_call() -> Result<(), BoxError> {
    run_intergration(async move |proxy_address| {
        let url = format!("http://{}/helloworld.Greeter/SayHello", proxy_address);
        let req_msg = HelloRequest {
            name: "Alice".to_string(),
        };

        let req = Request::post(url)
            .header("content-type", "application/grpc")
            .body(Full::<Bytes>::from(message_to_frame(&req_msg).freeze()))
            .unwrap();

        let client = Client::builder(TokioExecutor::new()).build_http();

        let res = client.request(req).await.unwrap();

        let res = res.map(|body| incoming_to_stream_body(body));
        assert_eq!(res.status(), 200);
        let mut body = res.into_body();
        let messages: Vec<HelloReply> = parse_grpc_stream(body).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages.first().unwrap().message, "Hello Alice!");

        Ok(())
    })
    .await
}

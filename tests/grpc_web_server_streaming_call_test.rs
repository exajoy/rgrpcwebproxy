use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures_core::Stream;
use futures_util::StreamExt;
use http::uri::Authority;
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
use tonic::Request;
use tonic_web::GrpcWebClientLayer;

use rgrpcwebproxy::{
    forward, incoming_to_stream_body,
    test_support::{
        greeter::hello_world::{HelloReply, HelloRequest, greeter_client::GreeterClient},
        preparation::run_intergration,
        utils::{message_to_frame, parse_grpc_stream},
    },
};
use tower::BoxError;

#[tokio::test]
async fn test_grpc_web_server_streaming_call() -> Result<(), BoxError> {
    run_intergration(async move |proxy_address| {
        let client = hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build_http();

        let svc = tower::ServiceBuilder::new()
            .layer(GrpcWebClientLayer::new())
            .service(client);

        let mut client =
            GreeterClient::with_origin(svc, format!("http://{}", proxy_address).try_into()?);

        let request = HelloRequest {
            name: "Tonic".into(),
        };

        let res = client
            .say_hello_stream(Request::new(request))
            .await
            .unwrap();

        let mut stream = res.into_inner();
        let mut replies = Vec::<String>::new();
        while let Some(resp) = stream.next().await {
            match resp {
                Ok(msg) => {
                    replies.push(msg.message.clone());
                }
                Err(e) => {
                    break;
                }
            }
        }
        assert_eq!(
            replies,
            vec!["first ok".to_string(), "second ok".to_string(),]
        );

        Ok(())
    })
    .await
}

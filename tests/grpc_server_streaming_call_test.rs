use futures_util::StreamExt;
use rgrpcwebproxy::{
    start_proxy,
    test_support::{
        greeter::{
            MyGreeter,
            hello_world::{
                HelloRequest, greeter_client::GreeterClient, greeter_server::GreeterServer,
            },
        },
        preparation::run_intergration,
    },
};
use tokio::{
    sync::{
        oneshot::{self, Sender},
        watch,
    },
    task::JoinHandle,
};
use tonic::Request;
use tower::BoxError;

#[tokio::test]
async fn test_grpc_server_streaming_call() -> Result<(), BoxError> {
    run_intergration(async move |proxy_address| {
        let mut client = GreeterClient::connect(format!("http://{}", proxy_address))
            .await
            .unwrap();

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

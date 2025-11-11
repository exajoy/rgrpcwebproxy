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
use tokio::sync::oneshot;
use tonic::Request;
use tower::BoxError;

#[tokio::test]
async fn test_grpc_unary_call() -> Result<(), BoxError> {
    run_intergration(async move |proxy_address| {
        let mut client = GreeterClient::connect(format!("http://{}", proxy_address))
            .await
            .unwrap();

        let res = client
            .say_hello(Request::new(HelloRequest {
                name: "Alice".into(),
            }))
            .await
            .unwrap();

        assert_eq!(res.into_inner().message, "Hello Alice!");
        Ok(())
    })
    .await
}

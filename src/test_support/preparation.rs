use futures_util::StreamExt;
use tokio::{
    sync::{
        oneshot::{self, Sender},
        watch,
    },
    task::JoinHandle,
};
use tonic::Request;
use tower::BoxError;

use crate::{
    start_proxy,
    test_support::greeter::{MyGreeter, hello_world::greeter_server::GreeterServer},
};

pub async fn run_intergration<F, Fut>(call: F) -> Result<(), BoxError>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<(), BoxError>>,
{
    let mock = MyGreeter {};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let forward_address = listener.local_addr().unwrap().to_string();
    let (backend_shutdown_tx, backend_shutdown_rx) = oneshot::channel::<()>();
    // start mock server
    let backend_task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(GreeterServer::new(mock))
            .serve_with_incoming_shutdown(
                tokio_stream::wrappers::TcpListenerStream::new(listener),
                async {
                    backend_shutdown_rx.await.ok();
                },
            )
            .await
            .unwrap();
    });

    let (proxy_shutdown_tx, proxy_shutdown_rx) = tokio::sync::watch::channel(false);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_address = listener.local_addr().unwrap().to_string();
    let proxy_task = tokio::spawn(start_proxy(listener, forward_address, proxy_shutdown_rx));

    call(proxy_address).await.unwrap();

    proxy_shutdown_tx.send(true).unwrap();
    backend_shutdown_tx.send(()).unwrap();

    // wait until both tasks are finished
    // with shutdown server
    let _ = proxy_task.await.unwrap();
    backend_task.await.unwrap();

    Ok(())
}

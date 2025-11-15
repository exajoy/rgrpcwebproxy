use clap::Parser;
use griffin::{command::args::Args, start_proxy};
use tokio::net::TcpListener;
use tower::BoxError;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let args = Args::parse();
    let proxy_address = format!("{}:{}", args.proxy_host, args.proxy_port);

    let forward_authority = format!("{}:{}", args.forward_host, args.forward_port);
    let (_, shutdown_rx) = tokio::sync::watch::channel(false);
    let listener = TcpListener::bind(proxy_address).await?;

    start_proxy(listener, forward_authority, shutdown_rx).await
}

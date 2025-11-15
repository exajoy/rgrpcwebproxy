use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "server", about = "Run the server with options")]
pub struct Args {
    #[arg(long, default_value = "127.0.0.1", help = "Proxy server host")]
    pub proxy_host: String,
    #[arg(long, default_value_t = 8080, help = "Proxy server port")]
    pub proxy_port: u16,

    #[arg(long, default_value = "127.0.0.1", help = "Forward server host")]
    pub forward_host: String,

    #[arg(long, default_value_t = 3000, help = "Forward server port")]
    pub forward_port: u16,
}

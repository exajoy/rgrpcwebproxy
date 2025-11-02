use std::pin::Pin;

use http::HeaderValue;
use http::uri::Authority;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::client::conn::http2;
use hyper::client::conn::http2::SendRequest;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};

#[cfg(test)]
use mockall::automock;

use tokio::net::TcpStream;

pub type MaybeSendRequest =
    Result<SendRequest<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>>;
pub type MaybeResponse = Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>>;

/// make request from proxy to grpc server
#[derive(Clone)]
pub struct GrpcClient;

#[cfg_attr(test, automock)]
pub trait GrpcClientHandler {
    fn form_forward_url(
        &self,
        proxy_url: &str,
        forward_req_base_url: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
    fn send_request(
        &self,
        request_sender: SendRequest<Full<Bytes>>,
        request: Request<Full<Bytes>>,
    ) -> Pin<Box<dyn Future<Output = MaybeResponse> + Send>>;
    fn process_req(
        &self,
        forward_url: &str,
        proxy_req_headers: hyper::HeaderMap,
        proxy_req_body: Full<Bytes>,
    ) -> Pin<Box<dyn Future<Output = MaybeResponse> + Send>>;
    // make connection to grpc server
    // and prepare request sender
    fn connect(
        &self,
        authority: Authority,
    ) -> Pin<Box<dyn Future<Output = MaybeSendRequest> + Send>>;
}
impl GrpcClientHandler for GrpcClient {
    fn form_forward_url(
        &self,
        proxy_url: &str,
        forward_req_base_url: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let proxy_req_uri: hyper::Uri = proxy_url.parse()?;
        let path = proxy_req_uri.path();
        let forward_url = format!("{}{}", forward_req_base_url, path);
        Ok(forward_url)
    }
    fn connect(
        &self,
        authority: Authority,
    ) -> Pin<Box<dyn Future<Output = MaybeSendRequest> + Send>> {
        Box::pin(async move {
            // Get the host and the port
            // let host = uri.host().expect("uri has no host");
            // let port = uri.port_u16().unwrap_or(3000);

            // let address = format!("{}:{}", host, port);
            // println!("Forwarding to address: {}", address);
            // // let address = "127.0.0.1:3000";
            //
            // println!("Connecting to address: {}", url_str);

            // let root_store = RootCertStore {
            //     roots: TLS_SERVER_ROOTS.iter().cloned().collect(),
            // };
            // let mut tls_config = ClientConfig::builder()
            //     .with_root_certificates(root_store)
            //     .with_no_client_auth();
            //
            // tls_config.alpn_protocols.push(b"h2".to_vec());
            // // tls_config.alpn_protocols.push(b"http/1.1".to_vec());
            //
            // let tls_config = Arc::new(tls_config);
            // let connector = TlsConnector::from(tls_config);
            // let domain = ServerName::try_from("localhost").unwrap();
            // let tcp = TcpStream::connect("127.0.0.1:3000").await.unwrap();
            // println!("TCP connected");
            // let tls = connector.connect(domain, tcp).await.unwrap();
            //
            // println!("TLS connected 2");

            // let io = TokioIo::new(tls);

            // // The authority of our URL will be the hostname of the httpbin remote
            // ) -> hyper::Result<Response<Incoming>> {
            // Implementation of sending gRPC request
            // // Open a TCP connection to the remote host

            let stream = TcpStream::connect(authority.as_str()).await?;
            // Use an adapter to access something implementing `tokio::io` traits as if they implement
            // `hyper::rt` IO traits.
            let io = TokioIo::new(stream);

            let exec = TokioExecutor::new();
            let (req_sender, conn) = http2::Builder::new(exec).handshake(io).await?;

            // Spawn a task to poll the connection, driving the HTTP state
            tokio::task::spawn(async move {
                if let Err(err) = conn.await {
                    println!("Connection failed: {:?}", err);
                }
            });
            Ok(req_sender)
        })
    }
    fn send_request(
        &self,
        mut request_sender: SendRequest<Full<Bytes>>,
        request: Request<Full<Bytes>>,
    ) -> Pin<Box<dyn Future<Output = MaybeResponse> + Send>> {
        Box::pin(async move {
            let incoming_response = request_sender.send_request(request).await?;
            let (parts, body) = incoming_response.into_parts();
            let body_bytes = body.collect().await?;
            let full_response = Response::from_parts(parts, Full::from(body_bytes.to_bytes()));
            Ok(full_response)
        })
    }

    fn process_req(
        &self,
        full_forward_address: &str,
        proxy_req_headers: hyper::HeaderMap,
        proxy_req_body: Full<Bytes>,
    ) -> Pin<Box<dyn Future<Output = MaybeResponse> + Send>> {
        let client = self.clone();
        let full_forward_address = full_forward_address.to_string();
        Box::pin(async move {
            let uri = full_forward_address.to_string().parse::<hyper::Uri>()?;
            let authority = uri.authority().unwrap().clone();
            // Create an HTTP request with an empty body and a HOST header
            let mut forward_req = Request::builder()
                .method(hyper::Method::POST)
                .uri(uri)
                .body(proxy_req_body)?;
            forward_req.headers_mut().extend(proxy_req_headers.clone());
            forward_req
                .headers_mut()
                .insert(hyper::header::HOST, authority.as_str().parse().unwrap());
            forward_req.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                HeaderValue::from_static("application/grpc"),
            );
            forward_req
                .headers_mut()
                .remove(hyper::header::TRANSFER_ENCODING);
            forward_req
                .headers_mut()
                .remove(hyper::header::TRANSFER_ENCODING);

            forward_req
                .headers_mut()
                .remove(hyper::header::CONTENT_LENGTH);
            let request_sender = client.connect(authority.clone()).await?;
            let proxy_res = client.send_request(request_sender, forward_req).await?;
            Ok(proxy_res)
        })
    }
}

mod tests {

    #[test]
    fn form_forward_url() {
        use super::{GrpcClient, GrpcClientHandler};

        let grpc_client = GrpcClient;
        let proxy_url = "http://localhost:3000/helloworld.Greeter/SayHello";
        // .parse()
        // .unwrap();
        let forward_req_base_url = "http://localhost:8080";
        let forward_url = grpc_client
            .form_forward_url(proxy_url, forward_req_base_url)
            .unwrap();
        assert_eq!(
            forward_url,
            "http://localhost:8080/helloworld.Greeter/SayHello"
        );
    }
    // #[tokio::test]
    // async fn test_forward_req() {
    //     let mut mock_grpc_client = MockGrpcClientHandler::new();
    //     mock_grpc_client.expect_send_request().returning(|_, _| {
    //         Box::pin(async move {
    //             let response = Response::builder()
    //                 .status(200)
    //                 .body(Full::from(Bytes::from("response body")))?;
    //             Ok(response)
    //         })
    //     });
    //     mock_grpc_client.expect_connect().returning(|_| {
    //         Box::pin(async {
    //             let stream = TcpStream::connect("127.0.0.1:50051").await?;
    //             let io = TokioIo::new(stream);
    //             let exec = TokioExecutor::new();
    //             let (send_request, _) = http2::handshake(exec, io).await?;
    //             Ok(send_request)
    //         })
    //     });
    //
    //     let proxy_res = mock_grpc_client
    //         .forward_req(
    //             "http://localhost:8080/helloworld.Greeter/SayHello".to_string(),
    //             hyper::HeaderMap::new(),
    //             Full::from(Bytes::from("Test body")),
    //         )
    //         .await;
    //     // assert!(proxy_res.is_ok());
    // }
}

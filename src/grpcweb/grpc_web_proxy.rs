use std::sync::Arc;

use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response};

use crate::grpcweb::args::Args;
use crate::grpcweb::grpc_client::GrpcClient;
use crate::grpcweb::grpc_client::GrpcClientHandler;
use crate::grpcweb::grpc_web_server::GrpcWebServer;
use crate::grpcweb::grpc_web_server::GrpcWebServerHandler;

/// Main proxy action
/// hold all parameters and handlers
pub(crate) struct GrpcWebProxy<T: GrpcClientHandler, S: GrpcWebServerHandler> {
    pub proxy_address: String,
    pub forward_base_url: String,
    grpc_client: T,
    grpc_web_server: S,
}
impl<T: GrpcClientHandler, S: GrpcWebServerHandler> GrpcWebProxy<T, S> {
    pub async fn handle_grpc_web_request<F>(
        self: Arc<Self>,
        req: Request<F>,
    ) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>>
    where
        F: hyper::body::Body + Send + 'static,
        F::Data: Send,
        F::Error: std::error::Error + Send + Sync + 'static,
    {
        let proxy_url = req.uri().to_string();
        // we need to build full forward req url
        // by combining forward base url with
        // uri path of proxy address
        let forward_url = self
            .grpc_client
            .form_forward_url(&proxy_url, &self.forward_base_url)?;
        let headers = req.headers().clone();

        // convert incoming request to bytes
        let body_as_bytes = req.collect().await?.to_bytes();
        let body = Full::from(body_as_bytes.clone());

        // send request and
        // get response from grpc service
        let proxy_res = self
            .grpc_client
            .process_req(&forward_url, headers, body)
            .await?;

        // forward response to grpc web client
        let forward_response = self.grpc_web_server.return_res(proxy_res).await?;

        Ok(forward_response)
    }
}

impl GrpcWebProxy<GrpcClient, GrpcWebServer> {
    /// create GrpcWebProxy instance from Args
    pub fn from_args(args: &Args) -> Self {
        let proxy_address = format!("{}:{}", args.proxy_host, args.proxy_port);
        let forward_base_url = format!("http://{}:{}", args.forward_host, args.forward_port);
        let grpc_client = GrpcClient;
        let grpc_web_server = GrpcWebServer;
        GrpcWebProxy::<GrpcClient, GrpcWebServer> {
            proxy_address,
            forward_base_url,
            grpc_client,
            grpc_web_server,
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::grpcweb::{
        grpc_client::MockGrpcClientHandler, grpc_web_server::MockGrpcWebServerHandler,
    };

    use super::*;

    impl GrpcWebProxy<MockGrpcClientHandler, MockGrpcWebServerHandler> {
        fn with_mock(
            grpc_client: MockGrpcClientHandler,
            grpc_web_server: MockGrpcWebServerHandler,
        ) -> Self {
            GrpcWebProxy {
                proxy_address: "".to_string(),
                forward_base_url: "".to_string(),
                grpc_client,
                grpc_web_server,
            }
        }
    }
    #[tokio::test]
    async fn test_forward_request() {
        let mut mock_grpc_client_handler = MockGrpcClientHandler::new();
        let successful_res = Response::builder()
            .status(200)
            .body(Full::from(Bytes::from("response body")))
            .unwrap();

        mock_grpc_client_handler
            .expect_form_forward_url()
            .returning(|_, _| Ok("".to_string()));

        let successful_res_clone = successful_res.clone();
        mock_grpc_client_handler
            .expect_process_req()
            .returning(move |_, _, _| {
                let successful_res_clone = successful_res_clone.clone();
                Box::pin(async move { Ok(successful_res_clone) })
            });

        let mut mock_grpc_web_server_handler = MockGrpcWebServerHandler::new();

        let successful_res_clone = successful_res.clone();
        mock_grpc_web_server_handler
            .expect_return_res()
            .returning(move |_| {
                let successful_res_clone = successful_res_clone.clone();
                Ok(successful_res_clone.clone())
            });
        let grpc_web_proxy = Arc::new(GrpcWebProxy::with_mock(
            mock_grpc_client_handler,
            mock_grpc_web_server_handler,
        ));
        let request = Request::builder()
            .uri("/test/path")
            .body(Full::<Bytes>::from("Hello"))
            .unwrap();
        let response = grpc_web_proxy.handle_grpc_web_request(request).await;
        assert!(response.is_ok());
    }
}

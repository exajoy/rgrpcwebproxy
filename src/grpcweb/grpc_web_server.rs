#[cfg(test)]
use mockall::automock;

use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::Response;
use hyper::body::Bytes;

/// handle request from client
/// and return request to client
#[derive(Clone)]
pub struct GrpcWebServer;

#[cfg_attr(test, automock)]
pub(crate) trait GrpcWebServerHandler {
    /// return response from grpc to
    /// grpc web client
    async fn return_res(
        &self,
        original_res: Response<Full<Bytes>>,
    ) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>>;
}
impl GrpcWebServerHandler for GrpcWebServer {
    async fn return_res(
        &self,
        original_res: Response<Full<Bytes>>,
    ) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
        let status = original_res.status();
        let headers = original_res.headers().clone();

        let collected = original_res.collect().await?;
        let body_bytes = collected.to_bytes();

        // [TODO] need to process trailer here
        let mut new_res = Response::builder()
            .status(status)
            .body(Full::from(body_bytes))?;
        new_res.headers_mut().extend(headers.into_iter());
        Ok(new_res)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_return_res() {
        let gprc_web_server = super::GrpcWebServer;
        let original_res = hyper::Response::builder()
            .status(200)
            .body(http_body_util::Full::from("Test body"))
            .unwrap();
        let response = gprc_web_server.return_res(original_res).await;
        assert!(response.is_ok());
    }
}

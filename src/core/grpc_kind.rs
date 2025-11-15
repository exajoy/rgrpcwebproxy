use bytes::Bytes;
use http::{HeaderValue, Request};
use hyper::client::conn::http2;
use tower::BoxError;

use crate::core::stream_response::StreamResponse;
use crate::core::{grpc_kind_plain::GrpcKindPlain, grpc_kind_web::GrpcKindWeb};

pub enum GrpcKind {
    Web(GrpcKindWeb),
    Plain(GrpcKindPlain),
}
impl GrpcKind {
    pub fn from_content_type(content_type: &HeaderValue) -> Option<Self> {
        if content_type == "application/grpc" {
            Some(GrpcKind::Plain(GrpcKindPlain))
        } else if content_type == "application/grpc-web"
            || content_type == "application/grpc-web+proto"
        {
            Some(GrpcKind::Web(GrpcKindWeb))
        } else {
            None
        }
    }
    pub async fn forward<B>(
        self,
        mut sender: http2::SendRequest<B>,
        mut req: Request<B>,
    ) -> Result<StreamResponse, BoxError>
    where
        B: hyper::body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        match self {
            GrpcKind::Web(ref kind) => kind.modify_request(&mut req),
            GrpcKind::Plain(_) => {}
        }

        let res = sender
            .send_request(req)
            .await
            .map_err(Into::<BoxError>::into)?;

        match self {
            GrpcKind::Plain(ref kind) => Ok(kind.modify_response(res)),
            GrpcKind::Web(ref kind) => Ok(kind.modify_response(res)),
        }
    }
}

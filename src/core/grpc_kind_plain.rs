use async_stream::try_stream;
use http::Response;
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Incoming;

use crate::core::stream_response::StreamResponse;
pub struct GrpcKindPlain;

impl GrpcKindPlain {
    pub fn modify_response(&self, res: Response<Incoming>) -> StreamResponse {
        let forward_stream = try_stream! {
                let mut incoming = res.into_body();
            while let Some(frame) = incoming.frame().await {
                let frame = frame?;
                yield frame;
            }
        };
        StreamResponse::new(StreamBody::new(Box::pin(forward_stream)))
    }
}

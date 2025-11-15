use async_stream::try_stream;
use http::{HeaderValue, Request, Response};
use http_body::Frame;
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Incoming;

use crate::{
    core::stream_response::{DynStream, StreamResponse},
    trailers::Trailers,
};
pub struct GrpcKindWeb;
impl GrpcKindWeb {
    pub fn modify_request<B>(&self, req: &mut Request<B>)
    where
        B: hyper::body::Body,
    {
        req.headers_mut().insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/grpc"),
        );
        req.headers_mut().remove(hyper::header::CONTENT_LENGTH);
    }

    pub fn modify_response(&self, res: Response<Incoming>) -> StreamResponse {
        let (parts, mut body) = res.into_parts();

        let forward_stream = try_stream! {
            while let Some(frame) = body.frame().await {
                let frame = frame?;

                if let Some(trailers) = frame.trailers_ref() {
                    let t = Trailers::new(trailers.clone());
                    yield Frame::data(t.into_to_frame());
                } else {
                    yield frame;
                }
            }
        };

        let boxed: DynStream = Box::pin(forward_stream);
        let body = StreamBody::new(boxed);

        let mut res = Response::from_parts(parts, body);
        res.headers_mut().insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/grpc-web+proto"),
        );
        res
    }
}

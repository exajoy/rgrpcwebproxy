use bytes::Bytes;

use futures_core::Stream;
use http::Response;

use http_body::Frame;
use http_body_util::StreamBody;
use std::pin::Pin;

pub type DynStream = Pin<Box<dyn Stream<Item = Result<Frame<Bytes>, hyper::Error>> + Send>>;
pub type StreamResponse = Response<StreamBody<DynStream>>;

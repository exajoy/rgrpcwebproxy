use std::convert::Infallible;

use async_stream::try_stream;
use bytes::Bytes;
use http_body_util::{BodyExt, Full, StreamBody};
use prometheus::{
    CounterVec, Encoder, HistogramVec, TextEncoder, register_counter_vec, register_histogram_vec,
};

use crate::core::stream_response::StreamResponse;
#[derive(Clone)]
pub struct Metrics {
    pub requests_total: CounterVec,
    pub request_duration: HistogramVec,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            requests_total: register_counter_vec!(
                "http_requests_total",
                "Total number of HTTP requests handled",
                &["method", "path"]
            )
            .unwrap(),
            request_duration: register_histogram_vec!(
                "http_request_duration_seconds",
                "Request duration in seconds",
                &["method", "path"]
            )
            .unwrap(),
        }
    }

    pub fn render(&self) -> StreamResponse {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();

        let body = Full::<Bytes>::from(buffer);
        from_full_bytes(body)
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

pub fn from_full_bytes(mut body: Full<Bytes>) -> StreamResponse {
    let forward_stream = try_stream! {
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|e: Infallible| -> hyper::Error { match e {} })?;
            yield frame;
        }
    };

    StreamResponse::new(StreamBody::new(Box::pin(forward_stream)))
}

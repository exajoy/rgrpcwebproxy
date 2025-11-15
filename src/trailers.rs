use bytes::{BufMut, Bytes, BytesMut};
use http::HeaderMap;

const FRAME_HEADER_SIZE: usize = 5;
const GRPC_WEB_TRAILERS_BIT: u8 = 0b10000000;
pub struct Trailers {
    inner: HeaderMap,
}
impl Trailers {
    pub fn new(inner: HeaderMap) -> Self {
        Self { inner }
    }

    fn encode(self) -> Vec<u8> {
        self.inner.iter().fold(Vec::new(), |mut acc, (key, value)| {
            acc.put_slice(key.as_ref());
            acc.push(b':');
            acc.put_slice(value.as_bytes());
            acc.put_slice(b"\r\n");
            acc
        })
    }
    pub fn into_to_frame(self) -> Bytes {
        let trailers = self.encode();
        let len = trailers.len();
        assert!(len <= u32::MAX as usize);

        let mut frame = BytesMut::with_capacity(len + FRAME_HEADER_SIZE);
        frame.put_u8(GRPC_WEB_TRAILERS_BIT);
        frame.put_u32(len as u32);
        frame.put_slice(&trailers);

        frame.freeze()
    }
}

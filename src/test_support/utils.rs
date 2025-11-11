use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures_core::Stream;
use http_body::Frame;
use http_body_util::StreamBody;
use prost::Message;

use futures_util::StreamExt;
use prost::DecodeError;

pub async fn parse_grpc_stream<M>(
    mut body: StreamBody<impl Stream<Item = Result<Frame<Bytes>, hyper::Error>> + Send + Unpin>,
) -> Result<Vec<M>, DecodeError>
where
    M: Message + Default,
{
    let mut messages = Vec::new();
    let mut buf = BytesMut::new();

    while let Some(next) = body.next().await {
        let frame = next.expect("body frame");

        // Skip trailers
        if let Ok(data) = frame.into_data() {
            buf.extend_from_slice(&data);

            // keep parsing while enough data for header (5 bytes)
            while buf.len() >= 5 {
                let mut reader = &buf[..];
                let _compressed_flag = reader.get_u8();
                let msg_len = reader.get_u32() as usize;

                if buf.len() < 5 + msg_len {
                    // incomplete message — wait for more data
                    break;
                }

                let msg_bytes = &buf[5..5 + msg_len];
                let msg = M::decode(msg_bytes)?;
                messages.push(msg);

                buf.advance(5 + msg_len);
            }
        }
    }

    Ok(messages)
}

pub fn message_to_frame(message: &impl Message) -> BytesMut {
    let mut buf = BytesMut::new();
    message.encode(&mut buf).unwrap();

    let mut framed = BytesMut::new();
    framed.put_u8(0); // not compressed
    framed.put_u32(buf.len() as u32);
    framed.put_slice(&buf);
    framed
}

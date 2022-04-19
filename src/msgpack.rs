use bytes::{Buf, BytesMut};
use tokio::io;
use tokio_util::codec::Decoder;
use rmp_serde::decode::Error;

use crate::Rpc;

#[derive(Debug, Default)]
pub struct MessagePackDecoder;

impl Decoder for MessagePackDecoder {
    type Item = Rpc;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let (id, method, params): (u32, String, Vec<String>) = match rmp_serde::from_read(&mut std::io::Cursor::new(src).reader()) {
            Ok(frame) => frame,
            Err(Error::LengthMismatch(_)) => return Ok(None),
            Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err)),
        };

        Ok(Some(Rpc {
            method,
            params,
        }))
    }
}

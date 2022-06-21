use std::io::Cursor;

use bytes::{Buf, BytesMut};
use tokio::io;
use tokio_util::codec::Decoder;
use rmp_serde::decode::{Deserializer, Error};
use serde::Deserialize;

use crate::Rpc;

#[derive(Debug, Default)]
pub struct MessagePackDecoder;

impl Decoder for MessagePackDecoder {
    type Item = Rpc;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let cursor = Cursor::new(&src);

        let mut deserializer = Deserializer::new(cursor);

        let (id, method, params) = match <(u32, String, Vec<String>)>::deserialize(&mut deserializer) {
            Ok(frame) => {
                let position = deserializer.position() as usize;
                drop(deserializer);
                src.advance(position);
                frame
            }
            Err(Error::InvalidMarkerRead(e) | Error::InvalidDataRead(e)) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(Error::LengthMismatch(_)) => return Ok(None),
            Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err)),
        };

        Ok(Some(Rpc {
            method,
            params,
        }))
    }
}

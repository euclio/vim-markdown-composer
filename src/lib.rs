use serde::Deserialize;

mod msgpack;

pub use msgpack::MessagePackDecoder as Decoder;

/// Represents an RPC request.
///
/// Assumes that the request's parameters are always `String`s.
#[derive(Debug, Deserialize)]
pub struct Rpc {
    pub method: String,
    pub params: Vec<String>,
}

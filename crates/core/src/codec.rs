//! plan_ref:
//!   - 03_storage/authority#ledger-entry-format-contract
//!   - 07_network#server-ws-runtime
//!
//! Project-owned binary codec facade.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BinaryCodecError {
    #[error("postcard serialization failed: {0}")]
    Serialize(#[source] postcard::Error),
    #[error("postcard deserialization failed: {0}")]
    Deserialize(#[source] postcard::Error),
    #[error("postcard payload has {0} trailing bytes")]
    TrailingBytes(usize),
}

pub fn encode<T>(value: &T) -> Result<Vec<u8>, BinaryCodecError>
where
    T: Serialize + ?Sized,
{
    postcard::to_allocvec(value).map_err(BinaryCodecError::Serialize)
}

/// Return the exact postcard byte length without allocating an encoded copy.
pub(crate) fn encoded_size<T>(value: &T) -> Result<usize, BinaryCodecError>
where
    T: Serialize + ?Sized,
{
    postcard::experimental::serialized_size(value).map_err(BinaryCodecError::Serialize)
}

pub fn decode<T>(bytes: &[u8]) -> Result<T, BinaryCodecError>
where
    T: DeserializeOwned,
{
    let (value, remaining) = decode_prefix(bytes)?;
    if !remaining.is_empty() {
        return Err(BinaryCodecError::TrailingBytes(remaining.len()));
    }
    Ok(value)
}

pub fn decode_prefix<'de, T>(bytes: &'de [u8]) -> Result<(T, &'de [u8]), BinaryCodecError>
where
    T: Deserialize<'de>,
{
    postcard::take_from_bytes(bytes).map_err(BinaryCodecError::Deserialize)
}

//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Versioned WebSocket frame helpers.

use super::{ClientMessage, ServerMessage};
use crate::codec;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const WS_PROTOCOL_VERSION: u16 = 5;
pub const MIN_SUPPORTED_WS_PROTOCOL_VERSION: u16 = 5;
pub const MAX_WS_FRAME_BYTES: u64 = 16 * 1024 * 1024;
/// Bound fact-count allocation before a transfer payload is materialized.
pub const MAX_SYNC_FACTS_PER_PAYLOAD: u64 = 16 * 1024;
/// Bound encoded ledger bytes before encryption/frame serialization duplicates the payload.
pub const MAX_SYNC_FACT_BYTES_PER_PAYLOAD: u64 = MAX_WS_FRAME_BYTES;
pub const WS_FRAME_MAGIC: &[u8] = b"DEVEWSF4";
pub const MISSING_WS_FRAME_MAGIC: &str = "missing WS frame magic";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsFrameFormat {
    VersionedBinary,
    VersionedJsonText,
}

#[derive(Debug, Clone)]
pub struct DecodedClientMessage {
    pub message: ClientMessage,
    pub format: WsFrameFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientFrame {
    pub protocol_version: u16,
    pub message: ClientMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFrame {
    pub protocol_version: u16,
    pub message: ServerMessage,
}

#[derive(Serialize)]
struct ServerFrameRef<'a> {
    protocol_version: u16,
    message: &'a ServerMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct VersionEnvelope {
    protocol_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolFrameError {
    Decode(String),
    UnsupportedVersion {
        received: u16,
        min: u16,
        current: u16,
    },
}

impl ClientFrame {
    pub fn current(message: ClientMessage) -> Self {
        Self {
            protocol_version: WS_PROTOCOL_VERSION,
            message,
        }
    }
}

impl ServerFrame {
    pub fn current(message: ServerMessage) -> Self {
        Self {
            protocol_version: WS_PROTOCOL_VERSION,
            message,
        }
    }
}

pub fn encode_client_binary(message: &ClientMessage) -> Result<Vec<u8>, ProtocolFrameError> {
    encode_client_binary_with_version(message, WS_PROTOCOL_VERSION)
}

pub fn encode_client_binary_with_version(
    message: &ClientMessage,
    protocol_version: u16,
) -> Result<Vec<u8>, ProtocolFrameError> {
    encode_binary_frame(&ClientFrame {
        protocol_version,
        message: message.clone(),
    })
}

pub fn encode_server_binary(message: &ServerMessage) -> Result<Vec<u8>, ProtocolFrameError> {
    encode_server_binary_with_version(message, WS_PROTOCOL_VERSION)
}

/// Exact postcard payload size for the current server frame without cloning or encoding it.
pub fn server_binary_payload_size(message: &ServerMessage) -> Result<u64, ProtocolFrameError> {
    postcard::experimental::serialized_size(&ServerFrameRef {
        protocol_version: WS_PROTOCOL_VERSION,
        message,
    })
    .map(|size| size as u64)
    .map_err(codec_error)
}

pub fn encode_server_binary_with_version(
    message: &ServerMessage,
    protocol_version: u16,
) -> Result<Vec<u8>, ProtocolFrameError> {
    encode_binary_frame(&ServerFrame {
        protocol_version,
        message: message.clone(),
    })
}

pub fn decode_client_binary(bytes: &[u8]) -> Result<ClientMessage, ProtocolFrameError> {
    Ok(decode_client_binary_with_format(bytes)?.message)
}

pub fn decode_client_binary_with_format(
    bytes: &[u8],
) -> Result<DecodedClientMessage, ProtocolFrameError> {
    Ok(DecodedClientMessage {
        message: decode_client_binary_frame(bytes)?.message,
        format: WsFrameFormat::VersionedBinary,
    })
}

pub fn decode_server_binary(bytes: &[u8]) -> Result<ServerMessage, ProtocolFrameError> {
    let frame: ServerFrame = decode_required_binary_frame(bytes)?;
    Ok(frame.message)
}

pub fn decode_client_binary_frame(bytes: &[u8]) -> Result<ClientFrame, ProtocolFrameError> {
    decode_required_binary_frame(bytes)
}

pub fn decode_server_binary_frame(bytes: &[u8]) -> Result<ServerFrame, ProtocolFrameError> {
    decode_required_binary_frame(bytes)
}

pub fn decode_client_json(text: &str) -> Result<ClientMessage, ProtocolFrameError> {
    Ok(decode_client_json_with_format(text)?.message)
}

pub fn decode_client_json_with_format(
    text: &str,
) -> Result<DecodedClientMessage, ProtocolFrameError> {
    let envelope = serde_json::from_str::<VersionEnvelope>(text).map_err(json_decode_error)?;
    ensure_supported(envelope.protocol_version)?;
    let frame = serde_json::from_str::<ClientFrame>(text).map_err(json_decode_error)?;
    Ok(DecodedClientMessage {
        message: frame.message,
        format: WsFrameFormat::VersionedJsonText,
    })
}

pub fn decode_server_json(text: &str) -> Result<ServerMessage, ProtocolFrameError> {
    let envelope = serde_json::from_str::<VersionEnvelope>(text).map_err(json_decode_error)?;
    ensure_supported(envelope.protocol_version)?;
    serde_json::from_str::<ServerFrame>(text)
        .map(|frame| frame.message)
        .map_err(json_decode_error)
}

fn encode_binary_frame<T: Serialize>(frame: &T) -> Result<Vec<u8>, ProtocolFrameError> {
    let body = codec::encode(frame).map_err(codec_error)?;
    if body.len() as u64 > MAX_WS_FRAME_BYTES {
        return Err(ProtocolFrameError::Decode(format!(
            "WS frame payload exceeds {} bytes",
            MAX_WS_FRAME_BYTES
        )));
    }
    let mut bytes = Vec::with_capacity(WS_FRAME_MAGIC.len() + body.len());
    bytes.extend_from_slice(WS_FRAME_MAGIC);
    bytes.extend(body);
    Ok(bytes)
}

fn framed_payload(bytes: &[u8]) -> Option<&[u8]> {
    bytes
        .starts_with(WS_FRAME_MAGIC)
        .then(|| &bytes[WS_FRAME_MAGIC.len()..])
}

fn decode_required_binary_frame<T: DeserializeOwned>(
    bytes: &[u8],
) -> Result<T, ProtocolFrameError> {
    let payload = framed_payload(bytes)
        .ok_or_else(|| ProtocolFrameError::Decode(MISSING_WS_FRAME_MAGIC.to_string()))?;
    if payload.len() as u64 > MAX_WS_FRAME_BYTES {
        return Err(ProtocolFrameError::Decode(format!(
            "WS frame payload exceeds {} bytes",
            MAX_WS_FRAME_BYTES
        )));
    }
    ensure_supported(decode_binary_protocol_version(payload)?)?;
    decode_postcard(payload)
}

fn decode_binary_protocol_version(bytes: &[u8]) -> Result<u16, ProtocolFrameError> {
    codec::decode_prefix::<VersionEnvelope>(bytes)
        .map(|(envelope, _)| envelope.protocol_version)
        .map_err(codec_error)
}

fn decode_postcard<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ProtocolFrameError> {
    codec::decode(bytes).map_err(codec_error)
}

fn ensure_supported(version: u16) -> Result<(), ProtocolFrameError> {
    if (MIN_SUPPORTED_WS_PROTOCOL_VERSION..=WS_PROTOCOL_VERSION).contains(&version) {
        return Ok(());
    }
    Err(ProtocolFrameError::UnsupportedVersion {
        received: version,
        min: MIN_SUPPORTED_WS_PROTOCOL_VERSION,
        current: WS_PROTOCOL_VERSION,
    })
}

fn codec_error(error: impl std::error::Error) -> ProtocolFrameError {
    ProtocolFrameError::Decode(error.to_string())
}

fn json_decode_error(error: serde_json::Error) -> ProtocolFrameError {
    ProtocolFrameError::Decode(error.to_string())
}

impl fmt::Display for ProtocolFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(detail) => write!(f, "{detail}"),
            Self::UnsupportedVersion {
                received,
                min,
                current,
            } => write!(
                f,
                "unsupported WS protocol version {received}; supported range {min}..={current}"
            ),
        }
    }
}

impl std::error::Error for ProtocolFrameError {}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod sync_transfer_field_tests;

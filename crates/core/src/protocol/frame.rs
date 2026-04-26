//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Versioned WebSocket frame helpers.

use super::{ClientMessage, ServerMessage};
use bincode::Options;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const WS_PROTOCOL_VERSION: u16 = 2;
pub const MIN_SUPPORTED_WS_PROTOCOL_VERSION: u16 = 2;
pub const MAX_WS_FRAME_BYTES: u64 = 16 * 1024 * 1024;
pub const WS_FRAME_MAGIC: &[u8] = b"DEVEWSF2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsFrameFormat {
    VersionedBinary,
    VersionedJsonText,
    LegacyJsonText,
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
    encode_binary_frame(&ServerFrame::current(message.clone()))
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
    ensure_supported(frame.protocol_version)?;
    Ok(frame.message)
}

pub fn decode_client_binary_frame(bytes: &[u8]) -> Result<ClientFrame, ProtocolFrameError> {
    let frame: ClientFrame = decode_required_binary_frame(bytes)?;
    ensure_supported(frame.protocol_version)?;
    Ok(frame)
}

pub fn decode_server_binary_frame(bytes: &[u8]) -> Result<ServerFrame, ProtocolFrameError> {
    let frame: ServerFrame = decode_required_binary_frame(bytes)?;
    ensure_supported(frame.protocol_version)?;
    Ok(frame)
}

pub fn decode_client_json(text: &str) -> Result<ClientMessage, ProtocolFrameError> {
    Ok(decode_client_json_with_format(text)?.message)
}

pub fn decode_client_json_with_format(
    text: &str,
) -> Result<DecodedClientMessage, ProtocolFrameError> {
    if let Ok(frame) = serde_json::from_str::<ClientFrame>(text) {
        ensure_supported(frame.protocol_version)?;
        return Ok(DecodedClientMessage {
            message: frame.message,
            format: WsFrameFormat::VersionedJsonText,
        });
    }
    Ok(DecodedClientMessage {
        message: serde_json::from_str::<ClientMessage>(text).map_err(json_decode_error)?,
        format: WsFrameFormat::LegacyJsonText,
    })
}

pub fn decode_server_json(text: &str) -> Result<ServerMessage, ProtocolFrameError> {
    if let Ok(frame) = serde_json::from_str::<ServerFrame>(text) {
        ensure_supported(frame.protocol_version)?;
        return Ok(frame.message);
    }
    serde_json::from_str::<ServerMessage>(text).map_err(json_decode_error)
}

fn encode_binary_frame<T: Serialize>(frame: &T) -> Result<Vec<u8>, ProtocolFrameError> {
    let body = bincode_options()
        .serialize(frame)
        .map_err(bincode_decode_error)?;
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
        .ok_or_else(|| ProtocolFrameError::Decode("missing WS frame magic".to_string()))?;
    decode_bincode(payload)
}

fn decode_bincode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ProtocolFrameError> {
    bincode_options()
        .deserialize(bytes)
        .map_err(bincode_decode_error)
}

fn bincode_options() -> impl Options {
    bincode::DefaultOptions::new()
        .with_limit(MAX_WS_FRAME_BYTES)
        .with_fixint_encoding()
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

fn bincode_decode_error(error: impl std::error::Error) -> ProtocolFrameError {
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
#[path = "frame_test.rs"]
mod tests;

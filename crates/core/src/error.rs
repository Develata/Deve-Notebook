//! Unified error handling for Deve-Note.
//!
//! This module provides custom error types and result aliases for consistent
//! error handling across the codebase.

use std::fmt;

/// Custom error type for Deve-Note operations.
#[derive(Debug)]
pub enum DeveError {
    /// I/O error when reading/writing files.
    Io(std::io::Error),
    /// Database error from Ledger operations.
    Database(String),
    /// Serialization/deserialization error.
    Serialization(String),
    /// Document not found.
    NotFound(String),
    /// Invalid path or filename.
    InvalidPath(String),
    /// WebSocket connection error.
    WebSocket(String),
    /// General error with message.
    Other(String),
}

impl fmt::Display for DeveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeveError::Io(e) => write!(f, "I/O error: {}", e),
            DeveError::Database(msg) => write!(f, "Database error: {}", msg),
            DeveError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            DeveError::NotFound(path) => write!(f, "Not found: {}", path),
            DeveError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            DeveError::WebSocket(msg) => write!(f, "WebSocket error: {}", msg),
            DeveError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for DeveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DeveError::Io(e) => Some(e),
            _ => None,
        }
    }
}

// Conversions from common error types

impl From<std::io::Error> for DeveError {
    fn from(err: std::io::Error) -> Self {
        DeveError::Io(err)
    }
}

impl From<serde_json::Error> for DeveError {
    fn from(err: serde_json::Error) -> Self {
        DeveError::Serialization(err.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<bincode::Error> for DeveError {
    fn from(err: bincode::Error) -> Self {
        DeveError::Serialization(err.to_string())
    }
}

impl From<anyhow::Error> for DeveError {
    fn from(err: anyhow::Error) -> Self {
        DeveError::Other(err.to_string())
    }
}

/// Result type alias using DeveError.
pub type Result<T> = std::result::Result<T, DeveError>;

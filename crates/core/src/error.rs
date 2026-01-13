//! Deve-Note 统一错误处理模块
//!
//! 本模块提供自定义错误类型 `DeveError` 及结果别名，保证全项目错误处理的一致性。

use std::fmt;

/// Deve-Note 的自定义错误类型
#[derive(Debug)]
pub enum DeveError {
    /// I/O 读写错误
    Io(std::io::Error),
    /// 数据库 (Redb) 操作错误
    Database(String),
    /// 序列化/反序列化错误
    Serialization(String),
    /// 文档未找到
    NotFound(String),
    /// 非法路径或文件名
    InvalidPath(String),
    /// WebSocket 连接或通信错误
    WebSocket(String),
    /// 其他通用错误
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

/// 使用 DeveError 的 Result 类型别名
pub type Result<T> = std::result::Result<T, DeveError>;

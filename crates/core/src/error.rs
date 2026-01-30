// crates\core\src
//! Deve-Note 统一错误处理模块
//!
//! 本模块提供自定义错误类型 `DeveError` 及结果别名，保证全项目错误处理的一致性。
//!
//! ## 设计原则
//!
//! - **零分配错误路径**: 使用 `thiserror` 的 `#[from]` 实现零开销错误转换。
//! - **语义完整性**: 保留底层错误类型，允许上层精确匹配。
//! - **条件编译**: 后端专用错误 (redb, bincode) 仅在非 WASM 环境下启用。

use thiserror::Error;

/// Deve-Note 的自定义错误类型
///
/// 使用 `thiserror` 实现，支持零开销错误转换。
#[derive(Debug, Error)]
pub enum DeveError {
    /// I/O 读写错误
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// 数据库 (Redb) 操作错误 (仅后端)
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Database error: {0}")]
    Database(#[from] redb::Error),

    /// 数据库表错误 (仅后端)
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Database table error: {0}")]
    DatabaseTable(#[from] redb::TableError),

    /// 数据库事务错误 (仅后端)
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Database transaction error: {0}")]
    DatabaseTransaction(#[from] redb::TransactionError),

    /// 数据库提交错误 (仅后端)
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Database commit error: {0}")]
    DatabaseCommit(#[from] redb::CommitError),

    /// 数据库存储错误 (仅后端)
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Database storage error: {0}")]
    DatabaseStorage(#[from] redb::StorageError),

    /// JSON 序列化/反序列化错误
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Bincode 序列化错误 (仅后端)
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Bincode serialization error: {0}")]
    Bincode(#[from] bincode::Error),

    /// 文档未找到
    #[error("Not found: {0}")]
    NotFound(String),

    /// 非法路径或文件名
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// WebSocket 连接或通信错误
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// 其他通用错误
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for DeveError {
    fn from(err: anyhow::Error) -> Self {
        DeveError::Other(err.to_string())
    }
}

/// 使用 DeveError 的 Result 类型别名
pub type Result<T> = std::result::Result<T, DeveError>;

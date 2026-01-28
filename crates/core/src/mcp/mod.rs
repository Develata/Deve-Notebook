// crates/core/src/mcp/mod.rs
//! # MCP Stub (Model Context Protocol)
//!
//! 轻量级 MCP 预留接口：只定义配置与工具元数据，
//! 运行时调用由上层实现或未来扩展。

mod config;
mod executor;
mod manager;
mod protocol;

pub use config::{McpServerConfig, McpServerKind};
pub use executor::McpExecutor;
pub use manager::{McpManager, McpServerStatus, McpToolEntry};
pub use protocol::{McpCallResult, McpToolSpec};

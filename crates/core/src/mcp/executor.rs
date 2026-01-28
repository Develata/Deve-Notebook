// crates/core/src/mcp/executor.rs
//! # MCP Executor Trait
//!
//! 提供可插拔的 MCP 传输实现（stdio/http）。

use super::{McpCallResult, McpToolSpec};
use anyhow::Result;
use serde_json::Value;

pub trait McpExecutor: Send + Sync {
    fn list_tools(&self) -> Result<Vec<McpToolSpec>>;
    fn call_tool(&self, name: &str, args: Value) -> Result<McpCallResult>;
}

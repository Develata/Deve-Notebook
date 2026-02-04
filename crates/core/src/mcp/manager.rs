// crates/core/src/mcp/manager.rs
//! # MCP Manager (Stub)
//!
//! 仅提供工具注册与查询入口，避免引入重型 SDK。
//! 未来可扩展为 Stdio/SSE 客户端。

use super::{McpCallResult, McpExecutor, McpServerConfig, McpToolSpec};
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum McpServerStatus {
    Connected,
    Failed { reason: String },
    NotConfigured,
}

#[derive(Debug, Clone)]
pub struct McpToolEntry {
    pub server: String,
    pub tool: McpToolSpec,
}

#[derive(Default)]
pub struct McpManager {
    servers: HashMap<String, McpServerConfig>,
    tools: HashMap<String, Vec<McpToolSpec>>,
    executors: HashMap<String, Arc<dyn McpExecutor>>,
    statuses: HashMap<String, McpServerStatus>,
}

impl McpManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_server(&mut self, cfg: McpServerConfig) {
        let name = cfg.name().to_string();
        self.servers.insert(name.clone(), cfg);
        self.statuses
            .entry(name)
            .or_insert(McpServerStatus::NotConfigured);
    }

    pub fn register_executor(&mut self, name: &str, exec: Arc<dyn McpExecutor>) {
        self.executors.insert(name.to_string(), exec);
    }

    pub fn set_status(&mut self, name: &str, status: McpServerStatus) {
        self.statuses.insert(name.to_string(), status);
    }

    pub fn list_status(&self) -> Vec<(String, McpServerStatus)> {
        let mut out: Vec<(String, McpServerStatus)> = self
            .statuses
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    pub fn list_servers(&self) -> Vec<String> {
        let mut names: Vec<String> = self.servers.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn set_tools(&mut self, server: &str, tools: Vec<McpToolSpec>) {
        self.tools.insert(server.to_string(), tools);
    }

    pub fn list_tools(&self, server: &str) -> Vec<McpToolSpec> {
        self.tools.get(server).cloned().unwrap_or_default()
    }

    pub fn list_all_tools(&self) -> Vec<McpToolEntry> {
        let mut out = Vec::new();
        for (server, tools) in &self.tools {
            for tool in tools {
                out.push(McpToolEntry {
                    server: server.clone(),
                    tool: tool.clone(),
                });
            }
        }
        out
    }

    pub fn call_tool(&self, server: &str, tool: &str, args: Value) -> Result<McpCallResult> {
        let exec = self
            .executors
            .get(server)
            .ok_or_else(|| anyhow!("MCP executor not found: {}", server))?;
        exec.call_tool(tool, args)
    }
}

// apps/cli/src/server/mcp/mod.rs
//! # MCP Client Executors (CLI)
//!
//! 轻量实现：
//! - Local: stdio JSON-RPC
//! - Remote: HTTP JSON-RPC

mod http;
mod protocol;
mod sse;
mod stdio;

use anyhow::{Result, anyhow};
use deve_core::mcp::{McpExecutor, McpManager, McpServerConfig, McpServerStatus, McpToolSpec};
use serde_json::Value;
use std::sync::Arc;

pub fn register_mcp_servers(manager: &mut McpManager, configs: Vec<McpServerConfig>) {
    for cfg in configs {
        let name = cfg.name().to_string();
        let retries = cfg.retries(1);
        let backoff_ms = cfg.backoff_ms(200);
        let exec: Arc<dyn McpExecutor> = match &cfg {
            McpServerConfig::Local {
                command, args, env, ..
            } => Arc::new(stdio::StdioExecutor::new(
                command.clone(),
                args.clone(),
                env.clone(),
                cfg.timeout_ms(8000),
                retries,
                backoff_ms,
            )),
            McpServerConfig::Remote { url, headers, .. } => Arc::new(http::HttpExecutor::new(
                url.clone(),
                headers.clone(),
                cfg.timeout_ms(8000),
                retries,
                backoff_ms,
            )),
            McpServerConfig::RemoteSse { url, headers, .. } => Arc::new(sse::SseExecutor::new(
                url.clone(),
                headers.clone(),
                cfg.timeout_ms(8000),
                retries,
                backoff_ms,
            )),
        };

        manager.register_server(cfg);
        manager.register_executor(&name, exec.clone());

        match exec.list_tools() {
            Ok(tools) => {
                manager.set_tools(&name, tools);
                manager.set_status(&name, McpServerStatus::Connected);
            }
            Err(err) => {
                manager.set_status(
                    &name,
                    McpServerStatus::Failed {
                        reason: err.to_string(),
                    },
                );
                tracing::warn!("MCP list_tools failed for {}: {:?}", name, err);
            }
        }
    }
}

pub(crate) fn parse_tools(value: Value) -> Result<Vec<McpToolSpec>> {
    let tools_val = value.get("tools").cloned().unwrap_or(value);
    let arr = tools_val
        .as_array()
        .ok_or_else(|| anyhow!("Invalid MCP tools"))?;

    let mut tools = Vec::new();
    for t in arr {
        let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let desc = t
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let schema = t
            .get("inputSchema")
            .or_else(|| t.get("input_schema"))
            .cloned()
            .unwrap_or_else(|| serde_json::json!({"type":"object"}));
        tools.push(McpToolSpec {
            name: name.to_string(),
            description: desc,
            input_schema: schema,
        });
    }
    Ok(tools)
}

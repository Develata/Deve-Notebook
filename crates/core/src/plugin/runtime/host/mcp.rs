// crates/core/src/plugin/runtime/host/mcp.rs
//! # MCP 宿主函数 (Stub)
//!
//! **功能**: 向 Rhai 暴露 MCP 工具列表与调用接口。

use crate::mcp::{McpManager, McpServerStatus};
use crate::plugin::manifest::Capability;
use rhai::{Engine, EvalAltResult};
use std::sync::Arc;

/// 注册 MCP API
pub fn register_mcp_api(engine: &mut Engine, caps: Arc<Capability>, manager: Arc<McpManager>) {
    let caps_list_tools = caps.clone();
    let manager_list = manager.clone();
    engine.register_fn(
        "mcp_list_tools",
        move || -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            if !caps_list_tools.check_mcp() {
                return Err("Permission denied: MCP access not allowed by manifest.".into());
            }
            let tools = manager_list.list_all_tools();
            let items: Vec<serde_json::Value> = tools
                .into_iter()
                .map(|t| {
                    serde_json::json!({
                        "server": t.server,
                        "name": t.tool.name,
                        "description": t.tool.description,
                        "input_schema": t.tool.input_schema,
                    })
                })
                .collect();
            let json = serde_json::Value::Array(items);
            rhai::serde::to_dynamic(&json).map_err(|e| e.to_string().into())
        },
    );

    let caps_list_servers = caps.clone();
    let manager_status = manager.clone();
    engine.register_fn(
        "mcp_list_servers",
        move || -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            if !caps_list_servers.check_mcp() {
                return Err("Permission denied: MCP access not allowed by manifest.".into());
            }
            let items: Vec<serde_json::Value> = manager_status
                .list_status()
                .into_iter()
                .map(|(name, status)| {
                    let status_text = match status {
                        McpServerStatus::Connected => "connected",
                        McpServerStatus::Failed { .. } => "failed",
                        McpServerStatus::NotConfigured => "not_configured",
                    };
                    serde_json::json!({
                        "name": name,
                        "status": status_text,
                    })
                })
                .collect();
            let json = serde_json::Value::Array(items);
            rhai::serde::to_dynamic(&json).map_err(|e| e.to_string().into())
        },
    );

    let caps_call = caps.clone();
    let manager_call = manager.clone();
    engine.register_fn(
        "mcp_call_tool",
        move |server: &str,
              name: &str,
              args: rhai::Dynamic|
              -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            if !caps_call.check_mcp() {
                return Err("Permission denied: MCP access not allowed by manifest.".into());
            }
            let args_json: serde_json::Value =
                rhai::serde::from_dynamic(&args).map_err(|e| e.to_string())?;
            let res = manager_call
                .call_tool(server, name, args_json)
                .map_err(|e| e.to_string())?;
            rhai::serde::to_dynamic(&res.content).map_err(|e| e.to_string().into())
        },
    );
}

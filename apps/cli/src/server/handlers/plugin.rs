//! # Plugin Handler (插件处理器)
//!
//! **架构作用**:
//! 处理来自客户端的插件调用请求 (RPC)。
//!
//! **核心功能清单**:
//! - `handle_plugin_call`:
//!   1. 在 AppState 中查找目标插件。
//!   2. 转换参数格式 (JSON -> Rhai)。
//!   3. 执行 Runtime 调用。
//!   4. 转换结果格式 (Rhai -> JSON) 并广播响应。
//!
//! **类型**: Plugin MAY (插件可选)

use std::sync::Arc;
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use crate::server::AppState;

pub async fn handle_plugin_call(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    req_id: String,
    plugin_id: String,
    fn_name: String,
    args: Vec<serde_json::Value>,
) {
    // 1. Find Plugin
    let plugin = state.plugins.iter().find(|p| p.manifest().id == plugin_id);
    
    if let Some(plugin) = plugin {
        // 2. Convert Args (JSON -> Dynamic)
        // Rhai's serde feature handles this usually via rhai::serde::to_dynamic
        // But we need to map over the vector.
        let rhai_args: Vec<rhai::Dynamic> = args.into_iter()
            .map(|v| rhai::serde::to_dynamic(&v).unwrap_or(rhai::Dynamic::UNIT))
            .collect();

        // 3. Call
        match plugin.call(&fn_name, rhai_args) {
            Ok(result) => {
                // 4. Convert Result (Dynamic -> JSON)
                let json_result: serde_json::Value = rhai::serde::from_dynamic(&result)
                    .unwrap_or(serde_json::Value::Null);

                let _ = tx.send(ServerMessage::PluginResponse {
                    req_id,
                    result: Some(json_result),
                    error: None,
                });
            },
            Err(e) => {
                let _ = tx.send(ServerMessage::PluginResponse {
                    req_id,
                    result: None,
                    error: Some(format!("Runtime Error: {}", e)),
                });
            }
        }
    } else {
        let _ = tx.send(ServerMessage::PluginResponse {
            req_id,
            result: None,
            error: Some(format!("Plugin '{}' not found", plugin_id)),
        });
    }
}

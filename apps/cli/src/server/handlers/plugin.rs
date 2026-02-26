// apps/cli/src/server/handlers/plugin.rs
//! # 插件处理器 (Plugin Handler)
//!
//! 处理来自客户端的插件调用请求 (RPC)

use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::plugin::runtime::chat_stream::{ChatStreamScope, ChatStreamSink};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use tokio::task::block_in_place;

/// 处理插件调用
pub async fn handle_plugin_call(
    state: &Arc<AppState>,
    ch: &DualChannel,
    req_id: String,
    plugin_id: String,
    fn_name: String,
    args: Vec<serde_json::Value>,
) {
    handle_plugin_call_with_plugins(&state.plugins, ch, req_id, plugin_id, fn_name, args).await
}

pub async fn handle_plugin_call_with_plugins(
    plugins: &[Box<dyn deve_core::plugin::runtime::PluginRuntime>],
    ch: &DualChannel,
    req_id: String,
    plugin_id: String,
    fn_name: String,
    args: Vec<serde_json::Value>,
) {
    // Agent Bridge 拦截: 绕过 Rhai 插件，直接调用外部 CLI
    if plugin_id == "agent-bridge" {
        crate::server::agent_bridge::handle_agent_chat(ch, req_id, args).await;
        return;
    }

    let plugin = plugins.iter().find(|p| p.manifest().id == plugin_id);

    if let Some(plugin) = plugin {
        let rhai_args: Vec<rhai::Dynamic> = args
            .into_iter()
            .map(|v| rhai::serde::to_dynamic(&v).unwrap_or(rhai::Dynamic::UNIT))
            .collect();

        let ch_for_stream = ch.clone();
        let stream_sink = ChatStreamSink::new(move |msg| ch_for_stream.unicast(msg));
        let call_result = block_in_place(|| {
            let _scope = ChatStreamScope::new(stream_sink);
            plugin.call(&fn_name, rhai_args)
        });

        match call_result {
            Ok(result) => {
                let json_result: serde_json::Value =
                    rhai::serde::from_dynamic(&result).unwrap_or(serde_json::Value::Null);
                ch.unicast(ServerMessage::PluginResponse {
                    req_id,
                    result: Some(json_result),
                    error: None,
                });
            }
            Err(e) => {
                ch.unicast(ServerMessage::PluginResponse {
                    req_id,
                    result: None,
                    error: Some(format!("Runtime Error: {}", e)),
                });
            }
        }
    } else {
        ch.unicast(ServerMessage::PluginResponse {
            req_id,
            result: None,
            error: Some(format!("Plugin '{}' not found", plugin_id)),
        });
    }
}

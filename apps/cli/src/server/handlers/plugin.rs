// apps/cli/src/server/handlers/plugin.rs
//! plan_ref:
//!   - 16_ai_agent#trusted-agent-bridge
//!   - 19_plugins#plugin-runtime-boundary
//!
//! # 插件处理器 (Plugin Handler)
//!
//! 处理来自客户端的插件调用请求 (RPC)

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::plugin_response::{
    send_plugin_capability_denied, send_plugin_invalid_message, send_plugin_request_failed,
    send_plugin_result, send_plugin_runtime_error, send_plugin_serialization_error,
    send_plugin_unknown_plugin, send_plugin_unsupported_message,
};
use deve_core::plugin::runtime::chat_stream::{ChatStreamScope, ChatStreamSink};
use deve_core::plugin::runtime::host::PluginHostServerError;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use tokio::task::block_in_place;

/// 处理插件调用
pub async fn handle_plugin_call(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope_nonce: Option<u64>,
    req_id: String,
    plugin_id: String,
    fn_name: String,
    args: Vec<serde_json::Value>,
) {
    handle_plugin_call_with_plugins_and_scope(
        &state.plugins,
        ch,
        scope_nonce,
        req_id,
        plugin_id,
        fn_name,
        args,
    )
    .await
}

pub async fn handle_plugin_call_with_plugins(
    plugins: &[Box<dyn deve_core::plugin::runtime::PluginRuntime>],
    ch: &DualChannel,
    req_id: String,
    plugin_id: String,
    fn_name: String,
    args: Vec<serde_json::Value>,
) {
    handle_plugin_call_with_plugins_and_scope(plugins, ch, None, req_id, plugin_id, fn_name, args)
        .await
}

async fn handle_plugin_call_with_plugins_and_scope(
    plugins: &[Box<dyn deve_core::plugin::runtime::PluginRuntime>],
    ch: &DualChannel,
    scope_nonce: Option<u64>,
    req_id: String,
    plugin_id: String,
    fn_name: String,
    args: Vec<serde_json::Value>,
) {
    if !is_plugin_rpc_allowed(&plugin_id, &fn_name) {
        send_plugin_unsupported_message(
            ch,
            &req_id,
            format!("Plugin function is not public: {plugin_id}::{fn_name}"),
        );
        return;
    }

    // Agent Bridge 拦截: 绕过 Rhai 插件，直接调用外部 CLI
    if plugin_id == "agent-bridge" {
        crate::server::agent_bridge::handle_agent_chat(ch, req_id, args).await;
        return;
    }
    if plugin_id == "ai-chat"
        && (!crate::server::ai_chat::is_native_ai_enabled()
            || !crate::server::ai_chat::is_native_ai_runtime_registered())
    {
        send_plugin_capability_denied(
            ch,
            &req_id,
            crate::server::ai_chat::NATIVE_AI_DISABLED_ERROR,
        );
        finish_chat(ch, &req_id);
        return;
    }

    let plugin = plugins.iter().find(|p| p.manifest().id == plugin_id);

    if let Some(plugin) = plugin {
        let rhai_args = match json_args_to_dynamic(args) {
            Ok(args) => args,
            Err(detail) => {
                send_plugin_invalid_message(ch, &req_id, detail);
                return;
            }
        };

        let call_result = block_in_place(|| {
            let _scope = (plugin_id == "ai-chat").then(|| {
                let ch_for_stream = ch.clone();
                ChatStreamScope::new(ChatStreamSink::new(move |msg| ch_for_stream.unicast(msg)))
            });
            plugin.call(&fn_name, rhai_args)
        });

        match call_result {
            Ok(result) => match dynamic_result_to_json(result) {
                Ok(json_result) => {
                    if let Some(detail) = plugin_result_error_detail(&json_result) {
                        send_plugin_request_failed(ch, &req_id, detail);
                    } else {
                        send_plugin_result(ch, req_id, json_result);
                    }
                }
                Err(detail) => send_plugin_serialization_error(ch, &req_id, detail),
            },
            Err(error) => match error.downcast_ref::<PluginHostServerError>() {
                Some(server_error) => {
                    ch.send_protocol_error_with_scope_nonce(server_error.0.clone(), scope_nonce)
                }
                None => {
                    send_plugin_runtime_error(ch, &req_id, format!("Plugin runtime error: {error}"))
                }
            },
        }
    } else {
        send_plugin_unknown_plugin(ch, &req_id, format!("Plugin not found: {}", plugin_id));
    }
}

fn finish_chat(ch: &DualChannel, req_id: &str) {
    ch.unicast(ServerMessage::ChatChunk {
        req_id: req_id.to_string(),
        delta: None,
        finish_reason: Some("stop".to_string()),
    });
}

fn is_plugin_rpc_allowed(plugin_id: &str, fn_name: &str) -> bool {
    match plugin_id {
        "ai-chat" | "agent-bridge" => fn_name == "chat",
        _ => true,
    }
}

fn json_args_to_dynamic(args: Vec<serde_json::Value>) -> Result<Vec<rhai::Dynamic>, String> {
    args.into_iter()
        .enumerate()
        .map(|(idx, value)| {
            rhai::serde::to_dynamic(&value)
                .map_err(|err| format!("Failed to encode plugin arg[{idx}] into Rhai: {err}"))
        })
        .collect()
}

fn dynamic_result_to_json(result: rhai::Dynamic) -> Result<serde_json::Value, String> {
    rhai::serde::from_dynamic(&result)
        .map_err(|err| format!("Plugin returned non-JSON-serializable result: {err}"))
}

fn plugin_result_error_detail(result: &serde_json::Value) -> Option<String> {
    if result.get("type").and_then(|value| value.as_str()) != Some("error") {
        return None;
    }
    let detail = result
        .get("content")
        .or_else(|| result.get("detail"))
        .or_else(|| result.get("message"))
        .and_then(|value| value.as_str())
        .unwrap_or("Plugin returned an error")
        .trim();
    Some(if detail.is_empty() {
        "Plugin returned an error".to_string()
    } else {
        detail.to_string()
    })
}

#[cfg(test)]
mod tests;

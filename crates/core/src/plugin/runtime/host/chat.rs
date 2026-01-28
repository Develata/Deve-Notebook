// crates/core/src/plugin/runtime/host/chat.rs
//! # AI 聊天宿主函数
//!
//! **功能**: 提供 AI 聊天流式处理能力。
//! **安全**: 需通过 Capability 的 net 域名检查。

use crate::plugin::manifest::Capability;
use crate::plugin::runtime::chat_stream;
use rhai::{Engine, EvalAltResult};
use std::sync::Arc;

/// 从 URL 提取域名
fn extract_domain(url: &str) -> Option<&str> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let host = without_scheme.split('/').next()?;
    host.split(':').next()
}

/// 注册 AI 聊天 API
pub fn register_chat_api(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_chat = caps.clone();
    let caps_chat_tools = caps.clone();

    // API: ai_chat_stream(req_id, config, history) -> Dynamic
    engine.register_fn(
        "ai_chat_stream",
        move |req_id: &str,
              config: rhai::Dynamic,
              history: rhai::Dynamic|
              -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            let config_json: serde_json::Value =
                rhai::serde::from_dynamic(&config).map_err(|e| e.to_string())?;
            let history_json: serde_json::Value =
                rhai::serde::from_dynamic(&history).map_err(|e| e.to_string())?;

            let base_url = config_json
                .get("base_url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing base_url".to_string())?;
            let domain = extract_domain(base_url).ok_or_else(|| "Invalid base_url".to_string())?;
            if !caps_chat.check_net(domain) {
                return Err(format!(
                    "Permission denied: net access to '{}' is not allowed by manifest.",
                    domain
                )
                .into());
            }

            let handler = chat_stream::chat_stream_handler()
                .ok_or_else(|| "Chat stream handler not configured".to_string())?;
            let sink = chat_stream::current_chat_stream_sink()
                .ok_or_else(|| "Chat stream sink not configured".to_string())?;

            let request = chat_stream::ChatStreamRequest {
                req_id: req_id.to_string(),
                config: config_json,
                history: history_json,
                tools: None,
            };
            let response = handler.stream(request, sink).map_err(|e| e.to_string())?;

            let result_json = serde_json::to_value(&response).map_err(|e| e.to_string())?;
            rhai::serde::to_dynamic(&result_json).map_err(|e| e.to_string().into())
        },
    );

    // API: ai_chat_stream_with_tools(req_id, config, history, tools) -> Dynamic
    engine.register_fn(
        "ai_chat_stream_with_tools",
        move |req_id: &str,
              config: rhai::Dynamic,
              history: rhai::Dynamic,
              tools: rhai::Dynamic|
              -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            let config_json: serde_json::Value =
                rhai::serde::from_dynamic(&config).map_err(|e| e.to_string())?;
            let history_json: serde_json::Value =
                rhai::serde::from_dynamic(&history).map_err(|e| e.to_string())?;
            let tools_json: serde_json::Value =
                rhai::serde::from_dynamic(&tools).map_err(|e| e.to_string())?;

            let base_url = config_json
                .get("base_url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing base_url".to_string())?;
            let domain = extract_domain(base_url).ok_or_else(|| "Invalid base_url".to_string())?;
            if !caps_chat_tools.check_net(domain) {
                return Err(format!(
                    "Permission denied: net access to '{}' is not allowed by manifest.",
                    domain
                )
                .into());
            }

            let handler = chat_stream::chat_stream_handler()
                .ok_or_else(|| "Chat stream handler not configured".to_string())?;
            let sink = chat_stream::current_chat_stream_sink()
                .ok_or_else(|| "Chat stream sink not configured".to_string())?;

            let request = chat_stream::ChatStreamRequest {
                req_id: req_id.to_string(),
                config: config_json,
                history: history_json,
                tools: Some(tools_json),
            };
            let response = handler.stream(request, sink).map_err(|e| e.to_string())?;

            let result_json = serde_json::to_value(&response).map_err(|e| e.to_string())?;
            rhai::serde::to_dynamic(&result_json).map_err(|e| e.to_string().into())
        },
    );
}

// crates/core/src/plugin/runtime/host/chat.rs
//! # AI 聊天宿主函数
//!
//! **功能**: 提供 AI 聊天流式处理能力。
//! **安全**: 需通过 Capability 的 net 域名检查。
//!
//! ## Invariants
//! - 所有网络请求必须通过 `check_net(domain)` 权限校验
//! - Handler 与 Sink 必须在调用前由 Server 层注入

use crate::plugin::manifest::Capability;
use crate::plugin::runtime::chat_stream::{
    self, ChatStreamHandler, ChatStreamRequest, ChatStreamSink,
};
use rhai::{Dynamic, Engine, EvalAltResult};
use serde_json::Value;
use std::sync::Arc;

/// 验证结果类型别名，降低类型复杂度
type ValidatedContext = (Arc<dyn ChatStreamHandler>, ChatStreamSink, Value, Value);

/// 从 URL 提取域名
///
/// **Pre-condition**: url 格式为 `scheme://host[:port]/path`
/// **Post-condition**: 返回 host 部分（不含端口）
fn extract_domain(url: &str) -> Option<&str> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let host = without_scheme.split('/').next()?;
    host.split(':').next()
}

/// 验证请求并准备执行环境
///
/// **Pre-condition**: config 必须包含 base_url 字段
/// **Post-condition**: 返回经过权限校验的 (handler, sink, config_json, history_json)
fn validate_and_prepare(
    caps: &Capability,
    config: &Dynamic,
    history: &Dynamic,
) -> Result<ValidatedContext, Box<EvalAltResult>> {
    let config_json: Value = rhai::serde::from_dynamic(config).map_err(|e| e.to_string())?;
    let history_json: Value = rhai::serde::from_dynamic(history).map_err(|e| e.to_string())?;

    let base_url = config_json
        .get("base_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing base_url".to_string())?;

    let domain = extract_domain(base_url).ok_or_else(|| "Invalid base_url".to_string())?;

    if !caps.check_net(domain) {
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

    Ok((handler, sink, config_json, history_json))
}

/// 执行流式请求并序列化结果
///
/// **Pre-condition**: handler 与 sink 已正确初始化
/// **Post-condition**: 返回可被 Rhai 消费的 Dynamic 结果
fn execute_stream(
    handler: Arc<dyn ChatStreamHandler>,
    sink: ChatStreamSink,
    req_id: &str,
    config_json: Value,
    history_json: Value,
    tools: Option<Value>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let request = ChatStreamRequest {
        req_id: req_id.to_string(),
        config: config_json,
        history: history_json,
        tools,
    };

    let response = handler.stream(request, sink).map_err(|e| e.to_string())?;
    let result_json = serde_json::to_value(&response).map_err(|e| e.to_string())?;

    rhai::serde::to_dynamic(&result_json).map_err(|e| e.to_string().into())
}

/// 注册 AI 聊天 API
pub fn register_chat_api(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_stream = caps.clone();
    let caps_stream_tools = caps.clone();

    // API: ai_chat_stream(req_id, config, history) -> Dynamic
    engine.register_fn(
        "ai_chat_stream",
        move |req_id: &str,
              config: Dynamic,
              history: Dynamic|
              -> Result<Dynamic, Box<EvalAltResult>> {
            let (handler, sink, config_json, history_json) =
                validate_and_prepare(&caps_stream, &config, &history)?;
            execute_stream(handler, sink, req_id, config_json, history_json, None)
        },
    );

    // API: ai_chat_stream_with_tools(req_id, config, history, tools) -> Dynamic
    engine.register_fn(
        "ai_chat_stream_with_tools",
        move |req_id: &str,
              config: Dynamic,
              history: Dynamic,
              tools: Dynamic|
              -> Result<Dynamic, Box<EvalAltResult>> {
            let (handler, sink, config_json, history_json) =
                validate_and_prepare(&caps_stream_tools, &config, &history)?;
            let tools_json: Value = rhai::serde::from_dynamic(&tools).map_err(|e| e.to_string())?;
            execute_stream(
                handler,
                sink,
                req_id,
                config_json,
                history_json,
                Some(tools_json),
            )
        },
    );
}

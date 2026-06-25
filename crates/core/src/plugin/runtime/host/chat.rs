// crates/core/src/plugin/runtime/host/chat.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 19_plugins#plugin-runtime-boundary
//!
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
/// **Pre-condition**: url 格式为 `http(s)://host[:port]/path`
/// **Post-condition**: 返回 host 部分（不含端口）；无效 authority fail-closed。
fn extract_domain(url: &str) -> Option<&str> {
    let trimmed = url.trim();
    if trimmed.is_empty()
        || trimmed
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b'\\')
    {
        return None;
    }

    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))?;
    let authority_end = without_scheme
        .find(['/', '?', '#'])
        .unwrap_or(without_scheme.len());
    let authority = &without_scheme[..authority_end];
    if authority.is_empty() || authority.contains('@') {
        return None;
    }

    extract_host_from_authority(authority)
}

fn extract_host_from_authority(authority: &str) -> Option<&str> {
    if let Some(bracketed) = authority.strip_prefix('[') {
        let end = bracketed.find(']')?;
        let host = &bracketed[..end];
        let port_suffix = &bracketed[end + 1..];
        return (!host.is_empty() && valid_port_suffix(port_suffix)).then_some(host);
    }

    let (host, port_suffix) = authority
        .split_once(':')
        .map_or((authority, ""), |(host, port)| (host, port));
    if host.is_empty() || host.contains(':') || host.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return None;
    }
    if !port_suffix.is_empty()
        && (port_suffix.contains(':') || !port_suffix.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }
    Some(host)
}

fn valid_port_suffix(suffix: &str) -> bool {
    if suffix.is_empty() {
        return true;
    }
    let Some(port) = suffix.strip_prefix(':') else {
        return false;
    };
    !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit())
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

    // Reserved API: Native AI handlers must reject tools fail-closed; this remains
    // only for compatibility tests and future explicitly-gated runtimes.
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

#[cfg(test)]
mod tests {
    use super::extract_domain;

    #[test]
    fn extract_domain_accepts_http_hosts_and_strips_ports() {
        assert_eq!(
            extract_domain("https://api.openai.com/v1"),
            Some("api.openai.com")
        );
        assert_eq!(
            extract_domain("http://127.0.0.1:11434/v1"),
            Some("127.0.0.1")
        );
        assert_eq!(extract_domain("http://[::1]:11434/v1"), Some("::1"));
    }

    #[test]
    fn extract_domain_rejects_ambiguous_or_invalid_authority() {
        assert_eq!(extract_domain("api.openai.com/v1"), None);
        assert_eq!(extract_domain("https:///v1"), None);
        assert_eq!(extract_domain("https://:443/v1"), None);
        assert_eq!(
            extract_domain("https://api.openai.com@evil.example/v1"),
            None
        );
        assert_eq!(extract_domain("https://api.openai.com:abc/v1"), None);
        assert_eq!(extract_domain("http://::1/v1"), None);
        assert_eq!(
            extract_domain("https://api.openai.com\\evil.example/v1"),
            None
        );
    }
}

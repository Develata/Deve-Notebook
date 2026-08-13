// crates/core/src/plugin/runtime/host/chat.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 19_plugins#plugin-runtime-boundary
//!
//! # AI 聊天宿主函数
//!
//! **功能**: 提供 AI 聊天流式处理能力。
//! **安全**: Rhai 仅能提交聊天历史；provider 配置与网络权威由 Server 持有。
//!
//! ## Invariants
//! - Handler 与 Sink 必须在调用前由 Server 层注入

use crate::plugin::runtime::chat_stream::{
    self, ChatStreamHandler, ChatStreamRequest, ChatStreamSink,
};
use rhai::{Dynamic, Engine, EvalAltResult};
use serde_json::Value;
use std::sync::Arc;

/// 验证结果类型别名，降低类型复杂度
type ValidatedContext = (Arc<dyn ChatStreamHandler>, ChatStreamSink, Value);

/// 验证请求并准备执行环境
///
/// **Post-condition**: 返回 handler、当前 request sink 与序列化后的 history。
fn validate_and_prepare(history: &Dynamic) -> Result<ValidatedContext, Box<EvalAltResult>> {
    let history_json: Value = rhai::serde::from_dynamic(history).map_err(|e| e.to_string())?;

    let handler = chat_stream::chat_stream_handler()
        .ok_or_else(|| "Chat stream handler not configured".to_string())?;
    let sink = chat_stream::current_chat_stream_sink()
        .ok_or_else(|| "Chat stream sink not configured".to_string())?;

    Ok((handler, sink, history_json))
}

/// 执行流式请求并序列化结果
///
/// **Pre-condition**: handler 与 sink 已正确初始化
/// **Post-condition**: 返回可被 Rhai 消费的 Dynamic 结果
fn execute_stream(
    handler: Arc<dyn ChatStreamHandler>,
    sink: ChatStreamSink,
    req_id: &str,
    history_json: Value,
    tools: Option<Value>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let request = ChatStreamRequest {
        req_id: req_id.to_string(),
        history: history_json,
        tools,
    };

    let response = handler.stream(request, sink).map_err(|e| e.to_string())?;
    let result_json = serde_json::to_value(&response).map_err(|e| e.to_string())?;

    rhai::serde::to_dynamic(&result_json).map_err(|e| e.to_string().into())
}

/// 注册 AI 聊天 API
pub(super) fn register_chat_api(engine: &mut Engine) {
    // API: ai_chat_stream(req_id, history) -> Dynamic
    engine.register_fn(
        "ai_chat_stream",
        move |req_id: &str, history: Dynamic| -> Result<Dynamic, Box<EvalAltResult>> {
            let (handler, sink, history_json) = validate_and_prepare(&history)?;
            execute_stream(handler, sink, req_id, history_json, None)
        },
    );

    // Reserved API: Native AI handlers must reject tools fail-closed; this remains
    // only for compatibility tests and future explicitly-gated runtimes.
    engine.register_fn(
        "ai_chat_stream_with_tools",
        move |req_id: &str,
              history: Dynamic,
              tools: Dynamic|
              -> Result<Dynamic, Box<EvalAltResult>> {
            let (handler, sink, history_json) = validate_and_prepare(&history)?;
            let tools_json: Value = rhai::serde::from_dynamic(&tools).map_err(|e| e.to_string())?;
            execute_stream(handler, sink, req_id, history_json, Some(tools_json))
        },
    );
}

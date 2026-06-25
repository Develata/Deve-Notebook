// apps/cli/src/server/ai_chat/mod.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! # AI Chat Streaming (Server Runtime)
//!
//! **功能**: OpenAI 兼容的流式聊天实现。
//!
//! **模块结构**:
//! - `config`: 配置结构
//! - `types`: SSE 响应数据类型
//! - `sse_parser`: SSE 消息解析与工具调用构建
//! - `stream`: 流式请求执行
//!
//! **优化**:
//! - 全局 HTTP 客户端单例 (复用 TCP 连接池)
//! - 强类型 SSE 解析 (避免 serde_json::Value)

mod config;
mod sse_parser;
mod stream;
mod types;

use anyhow::{Result, anyhow};
use config::ChatConfig;
use deve_core::plugin::runtime::chat_stream::{
    ChatStreamHandler, ChatStreamRequest, ChatStreamResponse, ChatStreamSink,
};
use deve_core::plugin::runtime::provider::register_provider;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use stream::execute_stream;

const NATIVE_AI_TOOLS_DISABLED_ERROR: &str = "Native AI Chat tools are disabled by default";
pub const NATIVE_AI_DISABLED_ERROR: &str = "Native AI Chat disabled by config";

static NATIVE_AI_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn init_from_config(config: &deve_core::config::Config) {
    NATIVE_AI_ENABLED.store(config.ai.native_enabled, Ordering::Relaxed);
}

pub fn init_chat_stream_handler() -> Result<()> {
    if !is_native_ai_enabled() {
        tracing::warn!("{}", NATIVE_AI_DISABLED_ERROR);
        return Ok(());
    }
    let handler = Arc::new(AiChatStreamHandler);
    register_provider(handler)
}

pub fn is_native_ai_enabled() -> bool {
    NATIVE_AI_ENABLED.load(Ordering::Relaxed)
}

struct AiChatStreamHandler;

fn reject_native_tools(tools: &Option<Value>) -> Result<()> {
    if tools.is_some() {
        return Err(anyhow!(NATIVE_AI_TOOLS_DISABLED_ERROR));
    }
    Ok(())
}

impl ChatStreamHandler for AiChatStreamHandler {
    fn stream(
        &self,
        request: ChatStreamRequest,
        sink: ChatStreamSink,
    ) -> Result<ChatStreamResponse> {
        reject_native_tools(&request.tools)?;

        let config: ChatConfig = serde_json::from_value(request.config)
            .map_err(|e| anyhow!("Invalid AI config: {}", e))?;
        config.validate().map_err(|e| anyhow!("{}", e))?;

        let history = request
            .history
            .as_array()
            .ok_or_else(|| anyhow!("Chat history must be an array"))?
            .clone();

        let body = json!({
            "model": config.model.trim(),
            "messages": history,
            "stream": true,
            "max_tokens": config.max_tokens,
        });

        let req_id = request.req_id.clone();
        let endpoint = config.endpoint();
        let headers = config.headers.clone();
        let api_key = config.api_key.trim().to_string();

        tokio::runtime::Handle::current().block_on(async move {
            execute_stream(&req_id, &endpoint, &api_key, &headers, body, &sink).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn native_ai_rejects_request_tools_before_provider_call() {
        let handler = AiChatStreamHandler;
        let sink = ChatStreamSink::new(|_| {});
        let request = ChatStreamRequest {
            req_id: "req-tools".to_string(),
            config: json!(null),
            history: json!(null),
            tools: Some(json!([])),
        };

        let err = handler
            .stream(request, sink)
            .expect_err("native AI must fail closed when tools are supplied");

        assert_eq!(err.to_string(), NATIVE_AI_TOOLS_DISABLED_ERROR);
    }
}

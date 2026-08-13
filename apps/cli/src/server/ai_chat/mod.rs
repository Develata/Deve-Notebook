// apps/cli/src/server/ai_chat/mod.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! # AI Chat Streaming (Server Runtime)
//!
//! **功能**: Server-owned Native AI provider settings 与精确多协议流式实现。
//!
//! **模块结构**:
//! - `settings`: 配置来源、脱敏 API、原子持久化与 runtime registry
//! - `providers`: 三种 provider 的 request/SSE peer adapters
//! - `types`: 共享流事件类型
//! - `stream`: 流式请求执行
//!
//! **优化**:
//! - 全局 HTTP 客户端单例 (复用 TCP 连接池)
//! - 强类型 SSE 解析 (避免 serde_json::Value)

mod builtin_runtime;
mod providers;
pub(crate) mod settings;
mod sse_parser;
mod stream;
mod types;

use anyhow::{Result, anyhow};
use deve_core::plugin::runtime::chat_stream::{
    ChatStreamHandler, ChatStreamRequest, ChatStreamResponse, ChatStreamSink,
};
use deve_core::plugin::runtime::provider::register_provider;
use serde_json::Value;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use stream::execute_stream;

const NATIVE_AI_TOOLS_DISABLED_ERROR: &str = "Native AI Chat tools are disabled by default";
pub const NATIVE_AI_DISABLED_ERROR: &str = "Native AI Chat disabled by config";

static NATIVE_AI_ENABLED: AtomicBool = AtomicBool::new(true);
static NATIVE_AI_RUNTIME_REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);
static NATIVE_AI_STREAM_HANDLER_INIT: OnceLock<Result<(), String>> = OnceLock::new();

pub(crate) struct NativeAiRuntimeRegistration {
    registered: bool,
    #[cfg(test)]
    _provider_settings: Option<settings::ProviderSettingsRegistration>,
}

impl NativeAiRuntimeRegistration {
    pub(crate) fn from_plugins(
        plugins: &[Box<dyn deve_core::plugin::runtime::PluginRuntime>],
    ) -> Self {
        let registered = plugins
            .iter()
            .any(|plugin| plugin.manifest().id == builtin_runtime::NATIVE_AI_PLUGIN_ID);
        if registered {
            NATIVE_AI_RUNTIME_REGISTRATIONS.fetch_add(1, Ordering::AcqRel);
        }
        Self {
            registered,
            #[cfg(test)]
            _provider_settings: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn is_registered(&self) -> bool {
        self.registered
    }

    #[cfg(test)]
    fn registered_for_test() -> Self {
        NATIVE_AI_RUNTIME_REGISTRATIONS.fetch_add(1, Ordering::AcqRel);
        let runtime = Arc::new(settings::NativeAiProviderSettingsRuntime::ready_for_test());
        let registration = settings::register(runtime).expect("register test AI provider settings");
        Self {
            registered: true,
            _provider_settings: Some(registration),
        }
    }
}

impl Drop for NativeAiRuntimeRegistration {
    fn drop(&mut self) {
        if self.registered {
            NATIVE_AI_RUNTIME_REGISTRATIONS.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

pub(crate) fn assemble_runtime_plugins(
    external_plugins: Vec<Box<dyn deve_core::plugin::runtime::PluginRuntime>>,
) -> Result<Vec<Box<dyn deve_core::plugin::runtime::PluginRuntime>>> {
    builtin_runtime::assemble_runtime_plugins(external_plugins, is_native_ai_enabled())
}

pub(crate) fn assemble_runtime_plugins_with_policy(
    external_plugins: Vec<Box<dyn deve_core::plugin::runtime::PluginRuntime>>,
    native_ai_enabled: bool,
) -> Result<Vec<Box<dyn deve_core::plugin::runtime::PluginRuntime>>> {
    builtin_runtime::assemble_runtime_plugins(external_plugins, native_ai_enabled)
}

pub(crate) fn is_native_ai_runtime_registered() -> bool {
    NATIVE_AI_RUNTIME_REGISTRATIONS.load(Ordering::Acquire) > 0
}

pub(crate) fn is_native_ai_provider_ready() -> bool {
    settings::current()
        .and_then(|runtime| runtime.snapshot())
        .is_ok_and(|snapshot| !snapshot.api_key.is_empty())
}

#[cfg(test)]
pub(crate) fn register_native_ai_runtime_for_test() -> NativeAiRuntimeRegistration {
    NativeAiRuntimeRegistration::registered_for_test()
}

pub fn init_from_config(config: &deve_core::config::Config) {
    NATIVE_AI_ENABLED.store(config.ai.native_enabled, Ordering::Relaxed);
}

pub fn init_chat_stream_handler() -> Result<()> {
    if !is_native_ai_enabled() {
        tracing::warn!("{}", NATIVE_AI_DISABLED_ERROR);
        return Ok(());
    }
    NATIVE_AI_STREAM_HANDLER_INIT
        .get_or_init(|| {
            let handler = Arc::new(AiChatStreamHandler);
            register_provider(handler).map_err(|error| error.to_string())
        })
        .as_ref()
        .map(|_| ())
        .map_err(|error| anyhow!(error.clone()))
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

        let config = settings::current()?.snapshot()?;
        if config.api_key.is_empty() {
            return Err(anyhow!("Native AI API key is not configured"));
        }

        let history = request
            .history
            .as_array()
            .ok_or_else(|| anyhow!("Chat history must be an array"))?
            .clone();

        let req_id = request.req_id.clone();
        let prepared = providers::prepare(&config, history)?;

        tokio::runtime::Handle::current()
            .block_on(async move { execute_stream(&req_id, &config, prepared, &sink).await })
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
            history: json!(null),
            tools: Some(json!([])),
        };

        let err = handler
            .stream(request, sink)
            .expect_err("native AI must fail closed when tools are supplied");

        assert_eq!(err.to_string(), NATIVE_AI_TOOLS_DISABLED_ERROR);
    }

    #[test]
    fn embedded_backend_reinitializes_same_stream_handler() {
        init_chat_stream_handler().expect("first embedded backend generation");
        init_chat_stream_handler().expect("second embedded backend generation");
    }
}

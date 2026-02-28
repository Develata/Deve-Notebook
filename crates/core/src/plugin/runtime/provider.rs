// crates/core/src/plugin/runtime/provider.rs
//! # AiProvider 接口层
//!
//! **功能**: 定义标准化 AI 提供者接口，是 D.5 规范的实现。
//!
//! ## 设计决策
//! 规范原型 `trait AiProvider { fn send_message(...) -> Stream<Token> }` 为 pull-based，
//! 但 Rhai 脚本引擎要求同步阻塞调用，因此采用 push-based sink 模型。
//!
//! `ChatStreamHandler` 即为事实上的 AiProvider：
//! - `stream()` 方法接收请求，通过 `ChatStreamSink` 推送增量 token
//! - 返回 `ChatStreamResponse` 表示最终结果（纯文本或工具调用请求）
//!
//! 本模块提供语义明确的 type alias 和文档，使 plugin SDK 的接口意图更清晰。
//!
//! ## Invariants
//! 1. 全局只有一个 AiProvider 实例（通过 OnceLock 保证）
//! 2. AiProvider 必须在插件加载前注册
//! 3. AiProvider 的 `stream()` 调用期间，thread-local sink 必须处于活跃状态

use super::chat_stream::{ChatStreamHandler, chat_stream_handler, set_chat_stream_handler};
use anyhow::Result;
use std::sync::Arc;

/// AI 提供者接口 —— `ChatStreamHandler` 的语义别名。
///
/// 所有 AI 后端（OpenAI、Anthropic、本地模型等）均实现此 trait。
/// 当前唯一实现：`apps/cli/src/server/ai_chat::AiChatStreamHandler`。
pub type AiProvider = dyn ChatStreamHandler;

/// 注册全局 AiProvider（进程生命周期内只能调用一次）
pub fn register_provider(provider: Arc<AiProvider>) -> Result<()> {
    set_chat_stream_handler(provider)
}

/// 获取已注册的 AiProvider
pub fn get_provider() -> Option<Arc<AiProvider>> {
    chat_stream_handler()
}

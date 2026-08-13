// apps/cli/src/server/ai_chat/types.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! # SSE 响应数据结构
//!
//! **功能**: OpenAI 兼容的 SSE 流式响应强类型定义。

/// 解析后的 SSE 事件
#[derive(Debug)]
pub enum ParsedSseEvent {
    /// 文本内容增量
    ContentDelta(String),
    /// 工具调用增量。Native AI Chat 只需要检测并拒绝，不保留工具元数据。
    ToolCallDelta,
    /// 流结束
    Finished(String),
    /// 无有效内容
    Empty,
}

// apps/cli/src/server/ai_chat/types.rs
//! # SSE 响应数据结构
//!
//! **功能**: OpenAI 兼容的 SSE 流式响应强类型定义。

use serde::Deserialize;

/// OpenAI SSE 响应结构 (强类型，避免 serde_json::Value)
#[derive(Debug, Deserialize)]
pub struct SseResponse {
    pub choices: Vec<SseChoice>,
}

#[derive(Debug, Deserialize)]
pub struct SseChoice {
    pub delta: Option<SseDelta>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SseDelta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<SseToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
pub struct SseToolCallDelta {
    pub index: Option<usize>,
    pub id: Option<String>,
    pub function: Option<SseFunctionDelta>,
}

#[derive(Debug, Deserialize)]
pub struct SseFunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

/// 解析后的 SSE 事件
#[derive(Debug)]
pub enum ParsedSseEvent {
    /// 文本内容增量
    ContentDelta(String),
    /// 工具调用增量
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments: Option<String>,
    },
    /// 流结束
    Finished(String),
    /// 无有效内容
    Empty,
}

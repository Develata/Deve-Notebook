// crates/core/src/plugin/runtime/tools.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! # Reserved Tool Definitions for AI Function Calling
//!
//! Native AI Chat ships read-first. Tool schemas stay available for future
//! explicit opt-in runtimes, but no default tool is exposed.

use serde::{Deserialize, Serialize};

/// Tool definition following OpenAI's schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String, // Always "function" for now
    pub function: FunctionDefinition,
}

/// Function definition within a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>, // JSON Schema
}

/// A tool call requested by the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String, // "function"
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON string
}

/// Tool result to send back to AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub role: String, // Always "tool"
    pub content: String,
}

impl ToolResult {
    pub fn new(tool_call_id: &str, content: &str) -> Self {
        Self {
            tool_call_id: tool_call_id.to_string(),
            role: "tool".to_string(),
            content: content.to_string(),
        }
    }

    pub fn error(tool_call_id: &str, error: &str) -> Self {
        Self {
            tool_call_id: tool_call_id.to_string(),
            role: "tool".to_string(),
            content: format!("Error: {}", error),
        }
    }
}

/// Built-in tools available to Native AI.
///
/// Current contract: no default tools. Native AI must not silently gain file,
/// source-control, shell, or skill authority through this list.
pub fn builtin_tools() -> Vec<ToolDefinition> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::builtin_tools;

    #[test]
    fn native_ai_exposes_no_default_tools() {
        assert!(builtin_tools().is_empty());
    }
}

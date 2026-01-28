// crates/core/src/plugin/runtime/tools.rs
//! # Tool Definitions for AI Function Calling
//!
//! Defines the schema for tools that AI can invoke during conversation.
//! Compatible with OpenAI's Function Calling / Tools API.

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

/// Built-in tools available to AI
pub fn builtin_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "read_file".to_string(),
                description: "Read the contents of a file at the given path".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file path to read"
                        }
                    },
                    "required": ["path"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "git_status".to_string(),
                description: "List unstaged changes in Redb source control".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "git_diff".to_string(),
                description: "Show unified diff for a document path".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The document path to diff"
                        }
                    },
                    "required": ["path"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "git_add".to_string(),
                description: "Stage a document path in Redb source control".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The document path to stage"
                        }
                    },
                    "required": ["path"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "git_commit".to_string(),
                description: "Commit staged documents with a message".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Commit message"
                        }
                    },
                    "required": ["message"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_files".to_string(),
                description: "Search for files matching a glob pattern".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Glob pattern (e.g., '**/*.rs')"
                        }
                    },
                    "required": ["pattern"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "grep".to_string(),
                description: "Search for a regex pattern in files".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Regex pattern to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory to search in (default: current directory)"
                        }
                    },
                    "required": ["pattern"]
                })),
            },
        },
    ]
}

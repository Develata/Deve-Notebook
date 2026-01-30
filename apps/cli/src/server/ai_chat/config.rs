// apps/cli/src/server/ai_chat/config.rs
//! # AI Chat 配置
//!
//! **功能**: 强类型的 AI 聊天配置结构。

use serde::Deserialize;
use std::collections::HashMap;

/// AI 聊天配置 (强类型)
#[derive(Debug, Deserialize)]
pub struct ChatConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

fn default_max_tokens() -> u32 {
    4096
}

impl ChatConfig {
    /// 构建 API endpoint URL
    pub fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    /// 验证配置有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.api_key.trim().is_empty() {
            return Err("Missing AI API key".to_string());
        }
        Ok(())
    }
}

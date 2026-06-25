// apps/cli/src/server/ai_chat/config.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
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
        format!(
            "{}/chat/completions",
            self.base_url.trim().trim_end_matches('/')
        )
    }

    /// 验证配置有效性
    pub fn validate(&self) -> Result<(), String> {
        let base_url = self.base_url.trim();
        if base_url.is_empty() {
            return Err("Missing AI base_url".to_string());
        }
        if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
            return Err("AI base_url must start with http:// or https://".to_string());
        }
        let parsed_base_url = reqwest::Url::parse(base_url)
            .map_err(|_| "AI base_url must be a valid http(s) URL".to_string())?;
        if parsed_base_url.host_str().is_none() {
            return Err("AI base_url must be a valid http(s) URL".to_string());
        }
        if !parsed_base_url.username().is_empty()
            || parsed_base_url.password().is_some()
            || parsed_base_url.query().is_some()
            || parsed_base_url.fragment().is_some()
        {
            return Err("AI base_url must not include userinfo, query, or fragment".to_string());
        }
        if self.api_key.trim().is_empty() {
            return Err("Missing AI API key".to_string());
        }
        if self.model.trim().is_empty() {
            return Err("Missing AI model".to_string());
        }
        if self.max_tokens == 0 {
            return Err("AI max_tokens must be greater than zero".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ChatConfig;
    use std::collections::HashMap;

    fn config() -> ChatConfig {
        ChatConfig {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
            max_tokens: 4096,
            headers: HashMap::new(),
        }
    }

    #[test]
    fn endpoint_uses_trimmed_base_url() {
        let mut config = config();
        config.base_url = " https://api.openai.com/v1/ ".to_string();

        assert_eq!(
            config.endpoint(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn validate_rejects_incomplete_provider_config() {
        let mut missing_base_url = config();
        missing_base_url.base_url = " ".to_string();
        assert_eq!(
            missing_base_url.validate().unwrap_err(),
            "Missing AI base_url"
        );

        let mut unsupported_scheme = config();
        unsupported_scheme.base_url = "file:///tmp/provider".to_string();
        assert_eq!(
            unsupported_scheme.validate().unwrap_err(),
            "AI base_url must start with http:// or https://"
        );

        let mut missing_model = config();
        missing_model.model = " ".to_string();
        assert_eq!(missing_model.validate().unwrap_err(), "Missing AI model");

        let mut zero_max_tokens = config();
        zero_max_tokens.max_tokens = 0;
        assert_eq!(
            zero_max_tokens.validate().unwrap_err(),
            "AI max_tokens must be greater than zero"
        );
    }

    #[test]
    fn validate_rejects_malformed_provider_base_url() {
        for base_url in ["https://", "http://[::1"] {
            let mut config = config();
            config.base_url = base_url.to_string();

            assert_eq!(
                config
                    .validate()
                    .expect_err("malformed base_url must fail closed"),
                "AI base_url must be a valid http(s) URL"
            );
        }
    }

    #[test]
    fn validate_rejects_provider_base_url_with_request_components() {
        for base_url in [
            "https://user:pass@api.example.test/v1",
            "https://api.example.test/v1?beta=1",
            "https://api.example.test/v1#chat",
        ] {
            let mut config = config();
            config.base_url = base_url.to_string();

            assert_eq!(
                config
                    .validate()
                    .expect_err("base_url request components must fail closed"),
                "AI base_url must not include userinfo, query, or fragment"
            );
        }
    }
}

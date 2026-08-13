//! plan_ref:
//!   - 15_settings#native-ai-provider-settings
//!
//! Redacted Native AI provider settings HTTP intents.

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use web_sys::RequestCredentials;

use super::native_http::api_url;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiProviderProtocol {
    OpenaiChatCompletions,
    OpenaiResponses,
    AnthropicMessages,
}

impl AiProviderProtocol {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpenaiChatCompletions => "openai-chat-completions",
            Self::OpenaiResponses => "openai-responses",
            Self::AnthropicMessages => "anthropic-messages",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "openai-responses" => Self::OpenaiResponses,
            "anthropic-messages" => Self::AnthropicMessages,
            _ => Self::OpenaiChatCompletions,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct AiProviderSettings {
    pub provider: AiProviderProtocol,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub key_configured: bool,
    pub source: String,
    pub revision: u64,
    pub writable: bool,
}

#[derive(Clone, Serialize)]
pub struct ReplaceAiProviderSettings {
    pub expected_revision: u64,
    pub provider: AiProviderProtocol,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub api_key: Option<String>,
    pub clear_api_key: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiSettingsApiError {
    EnvironmentManaged,
    RevisionConflict,
    Invalid,
    Unavailable,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: String,
}

pub async fn fetch_ai_provider_settings() -> Result<AiProviderSettings, AiSettingsApiError> {
    let api = api_url("/api/ai/settings");
    let mut request = Request::get(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request
        .send()
        .await
        .map_err(|_| AiSettingsApiError::Unavailable)?;
    if !response.ok() {
        return Err(decode_error(response).await);
    }
    response
        .json()
        .await
        .map_err(|_| AiSettingsApiError::Unavailable)
}

pub async fn replace_ai_provider_settings(
    payload: &ReplaceAiProviderSettings,
) -> Result<AiProviderSettings, AiSettingsApiError> {
    let api = api_url("/api/ai/settings");
    let mut request = Request::put(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request
        .header("Content-Type", "application/json")
        .json(payload)
        .map_err(|_| AiSettingsApiError::Invalid)?
        .send()
        .await
        .map_err(|_| AiSettingsApiError::Unavailable)?;
    if !response.ok() {
        return Err(decode_error(response).await);
    }
    response
        .json()
        .await
        .map_err(|_| AiSettingsApiError::Unavailable)
}

async fn decode_error(response: gloo_net::http::Response) -> AiSettingsApiError {
    match response
        .json::<ErrorBody>()
        .await
        .ok()
        .map(|body| body.error)
    {
        Some(code) if code == "environment_managed" => AiSettingsApiError::EnvironmentManaged,
        Some(code) if code == "revision_conflict" => AiSettingsApiError::RevisionConflict,
        Some(code) if code == "invalid_settings" => AiSettingsApiError::Invalid,
        _ => AiSettingsApiError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_wire_values_are_exact() {
        assert_eq!(
            AiProviderProtocol::OpenaiChatCompletions.as_str(),
            "openai-chat-completions"
        );
        assert_eq!(
            AiProviderProtocol::OpenaiResponses.as_str(),
            "openai-responses"
        );
        assert_eq!(
            AiProviderProtocol::AnthropicMessages.as_str(),
            "anthropic-messages"
        );
    }
}

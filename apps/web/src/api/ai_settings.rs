//! plan_ref:
//!   - 13_i18n#i18n-error-code-catalog
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
enum AiSettingsErrorCode {
    #[serde(rename = "AI_SETTINGS_ENVIRONMENT_MANAGED")]
    EnvironmentManaged,
    #[serde(rename = "AI_SETTINGS_REVISION_CONFLICT")]
    RevisionConflict,
    #[serde(rename = "AI_SETTINGS_INVALID")]
    Invalid,
    #[serde(rename = "AI_SETTINGS_UNAVAILABLE")]
    Unavailable,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorBody {
    code: AiSettingsErrorCode,
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
    let status = response.status();
    let code = response
        .json::<ErrorBody>()
        .await
        .ok()
        .map(|body| body.code);
    map_error_response(status, code)
}

fn map_error_response(status: u16, code: Option<AiSettingsErrorCode>) -> AiSettingsApiError {
    match (status, code) {
        (409, Some(AiSettingsErrorCode::EnvironmentManaged)) => {
            AiSettingsApiError::EnvironmentManaged
        }
        (409, Some(AiSettingsErrorCode::RevisionConflict)) => AiSettingsApiError::RevisionConflict,
        (400, Some(AiSettingsErrorCode::Invalid)) => AiSettingsApiError::Invalid,
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

    #[test]
    fn ai_settings_error_body_accepts_only_closed_typed_code() {
        let cases = [
            (
                r#"{"code":"AI_SETTINGS_ENVIRONMENT_MANAGED"}"#,
                AiSettingsErrorCode::EnvironmentManaged,
            ),
            (
                r#"{"code":"AI_SETTINGS_REVISION_CONFLICT"}"#,
                AiSettingsErrorCode::RevisionConflict,
            ),
            (
                r#"{"code":"AI_SETTINGS_INVALID"}"#,
                AiSettingsErrorCode::Invalid,
            ),
            (
                r#"{"code":"AI_SETTINGS_UNAVAILABLE"}"#,
                AiSettingsErrorCode::Unavailable,
            ),
        ];
        for (json, expected) in cases {
            let body: ErrorBody = serde_json::from_str(json).expect("closed error code");
            assert_eq!(body.code, expected);
        }

        assert!(serde_json::from_str::<ErrorBody>(r#"{"code":"AI_SETTINGS_FUTURE"}"#).is_err());
        assert!(
            serde_json::from_str::<ErrorBody>(
                r#"{"code":"AI_SETTINGS_UNAVAILABLE","detail":"private"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn ai_settings_status_code_matrix_and_mismatch_are_fail_closed() {
        let cases = [
            (
                409,
                Some(AiSettingsErrorCode::EnvironmentManaged),
                AiSettingsApiError::EnvironmentManaged,
            ),
            (
                409,
                Some(AiSettingsErrorCode::RevisionConflict),
                AiSettingsApiError::RevisionConflict,
            ),
            (
                400,
                Some(AiSettingsErrorCode::Invalid),
                AiSettingsApiError::Invalid,
            ),
            (
                500,
                Some(AiSettingsErrorCode::Unavailable),
                AiSettingsApiError::Unavailable,
            ),
        ];
        for (status, code, expected) in cases {
            assert_eq!(map_error_response(status, code), expected);
        }

        assert_eq!(
            map_error_response(500, Some(AiSettingsErrorCode::RevisionConflict)),
            AiSettingsApiError::Unavailable
        );
        assert_eq!(
            map_error_response(409, None),
            AiSettingsApiError::Unavailable
        );
    }
}

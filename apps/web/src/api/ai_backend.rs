//! plan_ref:
//!   - 10_ai_agent#trusted-agent-bridge
//!
use gloo_net::http::Request;
use serde::Deserialize;

pub const AI_BACKEND_NATIVE: &str = "native";
pub const AI_BACKEND_TRUSTED_CLI: &str = "trusted-cli";
pub const AI_PLUGIN_NATIVE: &str = "ai-chat";
pub const AI_PLUGIN_TRUSTED_CLI: &str = "agent-bridge";

#[derive(Clone, Debug, Deserialize)]
pub struct AiBackendCapabilities {
    #[serde(default = "default_native_available")]
    pub native_available: bool,
    #[serde(default)]
    pub native_reason: Option<String>,
    #[serde(default)]
    pub trusted_cli_available: bool,
    #[serde(default)]
    pub trusted_cli_reason: Option<String>,
    #[serde(default = "default_effective_backend")]
    pub effective_backend: String,
    #[serde(default)]
    pub effective_backend_reason: Option<String>,
}

impl Default for AiBackendCapabilities {
    fn default() -> Self {
        Self {
            native_available: true,
            native_reason: None,
            trusted_cli_available: false,
            trusted_cli_reason: Some("external agent disabled".to_string()),
            effective_backend: AI_BACKEND_NATIVE.to_string(),
            effective_backend_reason: None,
        }
    }
}

fn default_native_available() -> bool {
    true
}

fn default_effective_backend() -> String {
    AI_BACKEND_NATIVE.to_string()
}

pub async fn fetch_ai_backend_capabilities() -> AiBackendCapabilities {
    let response = Request::get("/api/ai/backend-capabilities").send().await;
    match response {
        Ok(resp) if resp.ok() => resp
            .json::<AiBackendCapabilities>()
            .await
            .unwrap_or_default(),
        _ => AiBackendCapabilities::default(),
    }
}

pub fn ai_backend_to_plugin_id(backend: &str) -> &'static str {
    match backend {
        AI_BACKEND_TRUSTED_CLI => AI_PLUGIN_TRUSTED_CLI,
        _ => AI_PLUGIN_NATIVE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_product_backend_names_to_runtime_plugin_ids() {
        assert_eq!(ai_backend_to_plugin_id(AI_BACKEND_NATIVE), AI_PLUGIN_NATIVE);
        assert_eq!(
            ai_backend_to_plugin_id(AI_BACKEND_TRUSTED_CLI),
            AI_PLUGIN_TRUSTED_CLI
        );
        assert_eq!(ai_backend_to_plugin_id("unknown"), AI_PLUGIN_NATIVE);
    }
}

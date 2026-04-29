//! plan_ref:
//!   - 10_ai_agent#trusted-agent-bridge
//!
use gloo_net::http::Request;
use serde::Deserialize;

pub const AI_BACKEND_NATIVE: &str = "native";
pub const AI_BACKEND_TRUSTED_CLI: &str = "trusted-cli";
pub const AI_PLUGIN_NATIVE: &str = "ai-chat";
pub const AI_PLUGIN_TRUSTED_CLI: &str = "agent-bridge";

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AiBackendCapabilities {
    #[serde(default)]
    pub trusted_cli_available: bool,
    #[serde(default)]
    pub trusted_cli_reason: Option<String>,
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

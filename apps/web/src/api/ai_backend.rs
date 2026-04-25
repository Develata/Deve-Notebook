//! plan_ref:
//!   - 10_ai_agent#trusted-agent-bridge
//!
use gloo_net::http::Request;
use serde::Deserialize;

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

//! plan_ref:
//!   - 18_release#runtime-observability
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! Peripheral runtime assembly for observability, prewarm, and mesh connectors.

use crate::server::{AppState, ai_chat, metrics, p2p, prewarm};
use deve_core::config::P2pConfig;
use deve_core::ledger::RepoManager;
use std::sync::Arc;

pub(crate) fn init_observability_runtime() -> anyhow::Result<()> {
    ai_chat::init_chat_stream_handler()?;
    metrics::init_start_time();
    Ok(())
}

pub(crate) fn spawn_prewarm(repo: Arc<RepoManager>) {
    prewarm::spawn_prewarm(repo);
}

pub(crate) fn spawn_background_runtime_tasks(p2p_config: P2pConfig, app_state: Arc<AppState>) {
    metrics::spawn_broadcaster(app_state.clone());
    p2p::spawn_mesh_connectors(p2p_config, app_state);
}

#[cfg(feature = "search")]
pub(crate) fn search_available(profile: deve_core::config::AppProfile) -> bool {
    if profile == deve_core::config::AppProfile::LowSpec {
        tracing::info!("LowSpec profile: search service disabled");
        false
    } else {
        tracing::info!("Search baseline scan enabled");
        true
    }
}

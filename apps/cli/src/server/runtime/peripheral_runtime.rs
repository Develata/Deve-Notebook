//! plan_ref:
//!   - 18_release#runtime-observability
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! Peripheral runtime assembly for observability, prewarm, and mesh connectors.

use crate::server::{AppState, ai_chat, metrics, p2p, prewarm};
use deve_core::config::P2pConfig;
use deve_core::ledger::RepoManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;

const ABORT_JOIN_GRACE: Duration = Duration::from_millis(250);

pub(crate) fn init_observability_runtime() -> anyhow::Result<()> {
    ai_chat::init_chat_stream_handler()?;
    metrics::init_start_time();
    Ok(())
}

pub(crate) struct BackgroundRuntimeTasks {
    shutdown: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
}

impl BackgroundRuntimeTasks {
    pub(crate) async fn shutdown(mut self, timeout: Duration) -> anyhow::Result<()> {
        let _ = self.shutdown.send(true);
        let deadline = tokio::time::Instant::now() + timeout;
        let mut first_error = None;
        while let Some(mut task) = self.tasks.pop() {
            match tokio::time::timeout_at(deadline, &mut task).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    first_error.get_or_insert_with(|| {
                        anyhow::anyhow!("server background runtime task join failed: {error}")
                    });
                }
                Err(_) => {
                    task.abort();
                    let mut remaining = self.tasks.drain(..).collect::<Vec<_>>();
                    for task in &remaining {
                        task.abort();
                    }
                    remaining.push(task);
                    let _ = tokio::time::timeout(ABORT_JOIN_GRACE, async move {
                        for task in remaining.drain(..) {
                            let _ = task.await;
                        }
                    })
                    .await;
                    first_error.get_or_insert_with(|| {
                        anyhow::anyhow!("server background runtime shutdown timed out")
                    });
                    break;
                }
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl Drop for BackgroundRuntimeTasks {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        for task in self.tasks.drain(..) {
            task.abort();
        }
    }
}

pub(crate) fn spawn_background_runtime_tasks(
    p2p_config: P2pConfig,
    app_state: Arc<AppState>,
    repo: Arc<RepoManager>,
    prewarm_enabled: bool,
) -> BackgroundRuntimeTasks {
    let (shutdown, shutdown_rx) = watch::channel(false);
    let mut tasks = Vec::new();
    if prewarm_enabled {
        tasks.push(prewarm::spawn_prewarm(repo, shutdown_rx.clone()));
    }
    tasks.push(metrics::spawn_broadcaster(
        app_state.clone(),
        shutdown_rx.clone(),
    ));
    tasks.extend(p2p::spawn_mesh_connectors(
        p2p_config,
        app_state,
        shutdown_rx,
    ));
    BackgroundRuntimeTasks { shutdown, tasks }
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

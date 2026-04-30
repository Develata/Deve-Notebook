use super::{dynamic_result_to_json, handle_plugin_call_with_plugins, is_plugin_rpc_allowed};
use crate::server::channel::DualChannel;
use anyhow::Result;
use deve_core::plugin::manifest::PluginManifest;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use rhai::Dynamic;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, broadcast, mpsc};

static AI_CONFIG_LOCK: Mutex<()> = Mutex::const_new(());

struct ProbePlugin {
    called: Arc<AtomicBool>,
    manifest: PluginManifest,
}

impl ProbePlugin {
    fn new(id: &str, called: Arc<AtomicBool>) -> Self {
        Self {
            called,
            manifest: PluginManifest {
                id: id.to_string(),
                name: "Probe".to_string(),
                version: "0.0.0".to_string(),
                entry: "main.rhai".to_string(),
                capabilities: Default::default(),
            },
        }
    }
}

impl PluginRuntime for ProbePlugin {
    fn load(&mut self, _manifest: PluginManifest, _script: &str) -> Result<()> {
        Ok(())
    }

    fn call(&self, _fn_name: &str, _args: Vec<Dynamic>) -> Result<Dynamic> {
        self.called.store(true, Ordering::SeqCst);
        Ok("called".into())
    }

    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
}

#[test]
fn plugin_result_fails_closed_when_dynamic_is_not_json_serializable() {
    let result = rhai::FnPtr::new("hidden").expect("fn ptr").into();
    let err = dynamic_result_to_json(result).unwrap_err();
    assert!(err.contains("non-JSON-serializable"));
}

#[test]
fn bundled_ai_public_rpc_allows_only_chat() {
    assert!(is_plugin_rpc_allowed("ai-chat", "chat"));
    assert!(!is_plugin_rpc_allowed("ai-chat", "run_tool"));
    assert!(!is_plugin_rpc_allowed("ai-chat", "build_config"));
    assert!(is_plugin_rpc_allowed("agent-bridge", "chat"));
    assert!(!is_plugin_rpc_allowed("agent-bridge", "build_config"));
}

#[tokio::test]
async fn bundled_ai_handler_rejects_internal_rpc_before_runtime_call() {
    let called = Arc::new(AtomicBool::new(false));
    let plugins: Vec<Box<dyn PluginRuntime>> =
        vec![Box::new(ProbePlugin::new("ai-chat", called.clone()))];
    let (tx, _) = broadcast::channel(4);
    let (uni_tx, mut uni_rx) = mpsc::channel(4);
    let ch = DualChannel::new(tx, uni_tx);

    handle_plugin_call_with_plugins(
        &plugins,
        &ch,
        "req-1".to_string(),
        "ai-chat".to_string(),
        "_build_config".to_string(),
        vec![],
    )
    .await;

    assert!(!called.load(Ordering::SeqCst));
    match uni_rx.recv().await.expect("plugin response") {
        ServerMessage::PluginResponse {
            req_id,
            result,
            error,
        } => {
            assert_eq!(req_id, "req-1");
            assert!(result.is_none());
            let error = error.expect("unsupported error");
            assert_eq!(error.code, ServerErrorCode::PluginUnsupportedMessage);
            assert!(error.detail.unwrap_or_default().contains("not public"));
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[tokio::test]
async fn native_ai_disabled_blocks_ai_chat_rpc_and_finishes_chat() {
    let _guard = AI_CONFIG_LOCK.lock().await;
    let called = Arc::new(AtomicBool::new(false));
    let plugins: Vec<Box<dyn PluginRuntime>> =
        vec![Box::new(ProbePlugin::new("ai-chat", called.clone()))];
    let (tx, _) = broadcast::channel(4);
    let (uni_tx, mut uni_rx) = mpsc::channel(4);
    let ch = DualChannel::new(tx, uni_tx);
    let mut config = deve_core::config::Config::load();
    config.ai.native_enabled = false;
    crate::server::ai_chat::init_from_config(&config);

    handle_plugin_call_with_plugins(
        &plugins,
        &ch,
        "req-disabled".to_string(),
        "ai-chat".to_string(),
        "chat".to_string(),
        vec![],
    )
    .await;

    config.ai.native_enabled = true;
    crate::server::ai_chat::init_from_config(&config);
    assert!(!called.load(Ordering::SeqCst));

    let mut saw_error = false;
    let mut saw_finish = false;
    for _ in 0..2 {
        match uni_rx.recv().await.expect("server message") {
            ServerMessage::PluginResponse {
                req_id,
                result,
                error,
            } => {
                assert_eq!(req_id, "req-disabled");
                assert!(result.is_none());
                assert!(
                    error
                        .expect("plugin error")
                        .detail
                        .unwrap_or_default()
                        .contains(crate::server::ai_chat::NATIVE_AI_DISABLED_ERROR)
                );
                saw_error = true;
            }
            ServerMessage::ChatChunk {
                req_id,
                delta,
                finish_reason,
            } => {
                assert_eq!(req_id, "req-disabled");
                assert!(delta.is_none());
                assert_eq!(finish_reason.as_deref(), Some("stop"));
                saw_finish = true;
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
    assert!(saw_error);
    assert!(saw_finish);
}

use super::{ProbePlugin, handle_plugin_call_with_plugins};
use crate::server::channel::DualChannel;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{broadcast, mpsc};

#[test]
fn plugin_error_result_extracts_structured_detail() {
    let detail = super::super::plugin_result_error_detail(&serde_json::json!({
        "type": "error",
        "content": "provider missing"
    }));

    assert_eq!(detail.as_deref(), Some("provider missing"));
}

#[test]
fn plugin_error_result_ignores_text_payload() {
    assert!(
        super::super::plugin_result_error_detail(&serde_json::json!({
            "type": "text",
            "content": "ok"
        }))
        .is_none()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn plugin_error_result_is_sent_as_structured_error() {
    let called = Arc::new(AtomicBool::new(false));
    let plugins: Vec<Box<dyn PluginRuntime>> = vec![Box::new(ProbePlugin::with_json_result(
        "error-result-probe",
        called.clone(),
        serde_json::json!({
            "type": "error",
            "content": "provider missing"
        }),
    ))];
    let (tx, _) = broadcast::channel(4);
    let (uni_tx, mut uni_rx) = mpsc::channel(4);
    let ch = DualChannel::new(tx, uni_tx);

    handle_plugin_call_with_plugins(
        &plugins,
        &ch,
        "req-error-result".to_string(),
        "error-result-probe".to_string(),
        "probe".to_string(),
        vec![],
    )
    .await;

    match uni_rx.recv().await.expect("plugin response") {
        ServerMessage::PluginResponse {
            req_id,
            result,
            error,
        } => {
            assert_eq!(req_id, "req-error-result");
            assert!(result.is_none());
            let error = error.expect("structured plugin error");
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(error.detail.as_deref(), Some("provider missing"));
        }
        other => panic!("unexpected message: {other:?}"),
    }
    assert!(called.load(Ordering::SeqCst));
}

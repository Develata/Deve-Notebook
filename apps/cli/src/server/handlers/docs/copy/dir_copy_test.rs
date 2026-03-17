use super::ensure_clean_node_consistency;
use crate::server::channel::DualChannel;
use deve_core::ledger::node_check::NodeConsistencyReport;
use deve_core::models::{DocId, NodeId};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::{broadcast, mpsc};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_dirty_node_consistency_report() {
    let (tx, _rx) = broadcast::channel(8);
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(tx, uni_tx);

    assert!(!ensure_clean_node_consistency(
        &ch,
        &NodeConsistencyReport {
            missing_nodes: vec![(DocId::new(), "notes/missing.md".into())],
            orphan_nodes: vec![(NodeId::new(), "notes/orphan.md".into())],
        },
        None,
    ));

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("Node consistency dirty after copy")),
                "unexpected detail: {:?}",
                error.detail
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}

use std::sync::Arc;
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use deve_core::models::LedgerEntry;
use crate::server::AppState;

pub async fn handle_edit(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    doc_id: deve_core::models::DocId,
    op: deve_core::models::Op,
    client_id: u64,
) {
    let entry = LedgerEntry {
        doc_id,
        op: op.clone(),
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    
    match state.ledger.append_op(&entry) {
        Ok(seq) => {
            // Broadcast to ALL with Sequence and ClientId
            let _ = tx.send(ServerMessage::NewOp { 
                doc_id, 
                op, 
                seq,
                client_id 
            });
            
            // [Persistence via SyncManager]
            if let Err(e) = state.sync_manager.persist_doc(doc_id) {
                tracing::error!("SyncManager failed to persist doc {}: {:?}", doc_id, e);
            }
        }
        Err(e) => {
            tracing::error!("Failed to persist op: {:?}", e);
        }
    }
}

pub async fn handle_request_history(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    doc_id: deve_core::models::DocId,
) {
     if let Ok(entries) = state.ledger.get_ops(doc_id) {
         let ops: Vec<(u64, deve_core::models::Op)> = entries.into_iter()
             .map(|(seq, entry)| (seq, entry.op))
             .collect();
         
         let msg = ServerMessage::History { doc_id, ops };
         let _ = tx.send(msg);
     }
}

pub async fn handle_open_doc(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    doc_id: deve_core::models::DocId,
) {
    tracing::info!("OpenDoc Request for DocID: {}", doc_id);
    
    // [Reconciliation via SyncManager]
    if let Err(e) = state.sync_manager.reconcile_doc(doc_id) {
        tracing::error!("SyncManager reconcile failed: {:?}", e);
    }

    // Return Snapshot from Ledger (Truth)
    let entries_with_seq = state.ledger.get_ops(doc_id).unwrap_or_default();
    let ops: Vec<deve_core::models::LedgerEntry> = entries_with_seq.iter().map(|(_, entry)| entry.clone()).collect();
    let final_content = deve_core::state::reconstruct_content(&ops);
    let version = entries_with_seq.last().map(|(seq, _)| *seq).unwrap_or(0);
    
    let snapshot = ServerMessage::Snapshot { doc_id, content: final_content, version };
    let _ = tx.send(snapshot);
}

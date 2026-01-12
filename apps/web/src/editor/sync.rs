use leptos::prelude::*;
use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::models::{DocId, Op};
use crate::api::WsService;
use super::EditorStats;
use super::ffi::{applyRemoteContent, applyRemoteOp, getEditorContent};

pub fn handle_server_message(
    msg: ServerMessage,
    doc_id: DocId,
    client_id: u64,
    ws: &WsService,
    // Signals
    set_content: WriteSignal<String>,
    local_version: ReadSignal<u64>,
    set_local_version: WriteSignal<u64>,
    set_history: WriteSignal<Vec<(u64, Op)>>,
    is_playback: ReadSignal<bool>,
    set_playback_version: WriteSignal<u64>,
    on_stats: Option<Callback<EditorStats>>,
) {
    match msg {
         ServerMessage::Snapshot { doc_id: msg_doc_id, content: new_content, version } => {
             // Filter by DocId
             if msg_doc_id != doc_id { return; }
             
             leptos::logging::log!("Received Snapshot: {} chars, Ver: {}", new_content.len(), version);
             
             // Compute initial stats
             if let Some(cb) = on_stats {
                 let lines = new_content.lines().count();
                 let words = new_content.split_whitespace().count();
                 cb.run(EditorStats { chars: new_content.len(), words, lines });
             }

             applyRemoteContent(&new_content);
             set_content.set(new_content);
             set_local_version.set(version);
             
             // Initialize playback range
             set_playback_version.set(version);
             
             // Request History
             ws.send(ClientMessage::RequestHistory { doc_id });
         }
         ServerMessage::History { doc_id: msg_doc_id, ops } => {
             if msg_doc_id != doc_id { return; }
             leptos::logging::log!("Received History: {} ops", ops.len());
             set_history.set(ops);
         }
         ServerMessage::NewOp { doc_id: msg_doc_id, op, seq, client_id: origin_id } => {
             if msg_doc_id != doc_id { return; }
             
             let current_ver = local_version.get_untracked();
             if seq > current_ver {
                 // Filter Echoes!
                 if origin_id != client_id {
                     if let Ok(json) = serde_json::to_string(&op) {
                         applyRemoteOp(&json);
                     }
                     // Update local content signal and stats
                     let txt = getEditorContent();
                     if let Some(cb) = on_stats {
                         let lines = txt.lines().count();
                         let words = txt.split_whitespace().count();
                         cb.run(EditorStats { chars: txt.len(), words, lines });
                     }
                     set_content.set(txt);
                 }
                 set_local_version.set(seq);
                 
                 // Append to History signal if valid
                 set_history.update(|h| h.push((seq, op)));
                 
                 // Auto-advance playback if we are at the "head" (live)
                 if !is_playback.get_untracked() {
                    set_playback_version.set(seq);
                 }
             }
         }
         ServerMessage::SyncHello { peer_id, vector: _ } => {
             leptos::logging::log!("P2P Handshake from Peer: {}", peer_id);
         },
         ServerMessage::Pong => {
             // leptos::logging::log!("Pong received");
         },
         ServerMessage::SyncPush { ops } => {
             leptos::logging::log!("Received SyncPush: {} ops", ops.len());
             
             let mut max_seq = local_version.get_untracked();
             let mut applied_count = 0;

             for (seq, entry) in ops {
                 if entry.doc_id == doc_id {
                     if seq > max_seq {
                         if let Ok(json) = serde_json::to_string(&entry.op) {
                             applyRemoteOp(&json);
                             applied_count += 1;
                         }
                         max_seq = seq;
                         // History
                         set_history.update(|h| h.push((seq, entry.op)));
                     }
                 }
             }

             if applied_count > 0 {
                 let txt = getEditorContent();
                 if let Some(cb) = on_stats {
                     let lines = txt.lines().count();
                     let words = txt.split_whitespace().count();
                     cb.run(EditorStats { chars: txt.len(), words, lines });
                 }
                 set_content.set(txt);
                 set_local_version.set(max_seq);
                 
                 // Playback
                 if !is_playback.get_untracked() {
                    set_playback_version.set(max_seq);
                 }
             }
         }
         _ => {}
    }
}

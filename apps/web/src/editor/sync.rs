//! # Sync Logic (同步逻辑)
//!
//! **架构作用**:
//! 处理来自 WebSocket 的服务器消息 (`ServerMessage`)。
//! 分发快照、历史记录、新操作 (NewOp) 和 P2P 同步通知。
//! 负责更新本地文档状态、版本号和 CodeMirror 内容。

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
    // 信号
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
             // 按 DocId 过滤
             if msg_doc_id != doc_id { return; }
             
             leptos::logging::log!("Received Snapshot: {} chars, Ver: {}", new_content.len(), version);
             
             // 计算初始统计信息
             if let Some(cb) = on_stats {
                 let lines = new_content.lines().count();
                 let words = new_content.split_whitespace().count();
                 cb.run(EditorStats { chars: new_content.len(), words, lines });
             }

             applyRemoteContent(&new_content);
             set_content.set(new_content);
             set_local_version.set(version);
             
             // 初始化回放范围
             set_playback_version.set(version);
             
             // 请求历史记录
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
                 // 过滤回显 (Echoes)!
                 if origin_id != client_id {
                     if let Ok(json) = serde_json::to_string(&op) {
                         applyRemoteOp(&json);
                     }
                     // 更新本地内容信号和统计信息
                     let txt = getEditorContent();
                     if let Some(cb) = on_stats {
                         let lines = txt.lines().count();
                         let words = txt.split_whitespace().count();
                         cb.run(EditorStats { chars: txt.len(), words, lines });
                     }
                     set_content.set(txt);
                 }
                 set_local_version.set(seq);
                 
                 // 如果有效，追加到历史信号
                 set_history.update(|h| h.push((seq, op)));
                 
                 // 如果处于 "head" (实时) 状态，自动推进回放
                 if !is_playback.get_untracked() {
                    set_playback_version.set(seq);
                 }
             }
         }
         ServerMessage::SyncHello { peer_id, vector: _, .. } => {
             leptos::logging::log!("P2P Handshake from Peer: {}", peer_id);
         },
         ServerMessage::Pong => {
             // leptos::logging::log!("Pong received");
         },
         ServerMessage::SyncPush { ops } => {
             leptos::logging::log!("Received SyncPush: {} encrypted ops", ops.len());
             
             // TODO: Decrypt ops using RepoKey (requires Key Exchange or Password derivation)
             // Currently just logging as we don't have the shared key on the client yet.
             for enc_op in ops {
                 leptos::logging::warn!("Skipping encrypted op seq: {} (No RepoKey)", enc_op.seq);
             }
             
             // Placeholder for when we have the key:
             /*
             let mut max_seq = local_version.get_untracked();
             let mut applied_count = 0;

             for enc_op in ops {
                 let seq = enc_op.seq;
                 // let entry = repo_key.decrypt(&enc_op)?; 
                 
                 if entry.doc_id == doc_id {
                     if seq > max_seq {
                         if let Ok(json) = serde_json::to_string(&entry.op) {
                             applyRemoteOp(&json);
                             applied_count += 1;
                         }
                         max_seq = seq;
                         // 历史记录
                         set_history.update(|h| h.push((seq, entry.op)));
                     }
                 }
             }
             */
             // Placeholder for key availability
             // if applied_count > 0 { ... }
         }
         _ => {}
    }
}

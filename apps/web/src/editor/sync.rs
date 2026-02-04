// apps\web\src\editor
//! # Sync Logic (同步逻辑)
//!
//! **架构作用**:
//! 处理来自 WebSocket 的服务器消息 (`ServerMessage`)。
//! 分发快照、历史记录、新操作 (NewOp) 和 P2P 同步通知。
//! 负责更新本地文档状态、版本号和 CodeMirror 内容。

use super::EditorStats;
use super::ffi::{applyRemoteContent, applyRemoteOp, applyRemoteOpsBatch, getEditorContent};
use super::prefetch::{PrefetchConfig, apply_ops_in_batches};
use crate::api::WsService;
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ClientMessage, ServerMessage};
use leptos::prelude::*;

#[allow(clippy::too_many_arguments)]
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
    set_load_state: WriteSignal<String>,
    set_load_progress: WriteSignal<(usize, usize)>,
    set_load_eta_ms: WriteSignal<u64>,
    on_stats: Option<Callback<EditorStats>>,
) {
    match msg {
        ServerMessage::Snapshot {
            doc_id: msg_doc_id,
            content: new_content,
            base_seq,
            version,
            delta_ops,
        } => {
            // 按 DocId 过滤
            if msg_doc_id != doc_id {
                return;
            }

            let load_start = now_ms();

            leptos::logging::log!(
                "Received Snapshot: {} chars, Base: {}, Ver: {}, Pending: {}",
                new_content.len(),
                base_seq,
                version,
                delta_ops.len()
            );

            // 计算初始统计信息
            if let Some(cb) = on_stats {
                let lines = new_content.lines().count();
                let words = new_content.split_whitespace().count();
                cb.run(EditorStats {
                    chars: new_content.len(),
                    words,
                    lines,
                });
            }

            applyRemoteContent(&new_content);
            set_content.set(new_content);
            set_local_version.set(base_seq);

            // 初始化回放范围
            set_playback_version.set(base_seq);
            set_load_state.set("partial".to_string());
            set_load_progress.set((0, delta_ops.len()));
            set_load_eta_ms.set(0);

            if delta_ops.is_empty() {
                set_local_version.set(version);
                set_playback_version.set(version);
                set_load_state.set("ready".to_string());
                set_load_progress.set((0, 0));
                set_load_eta_ms.set(0);
                leptos::logging::log!(
                    "Snapshot load complete: doc={}, elapsed_ms={}",
                    doc_id,
                    (now_ms() - load_start) as u64
                );
                ws.send(ClientMessage::RequestHistory { doc_id });
                return;
            }

            let ws = ws.clone();

            let apply_batch = std::rc::Rc::new(move |batch: &[(u64, Op)]| {
                let ops_only: Vec<Op> = batch.iter().map(|(_, op)| op.clone()).collect();
                if let Ok(json) = serde_json::to_string(&ops_only) {
                    applyRemoteOpsBatch(&json);
                }
                if let Some((seq, _)) = batch.last() {
                    set_local_version.set(*seq);
                }
                set_history.update(|h| {
                    for (seq, op) in batch {
                        h.push((*seq, op.clone()));
                    }
                });
            });

            let elapsed_total = std::rc::Rc::new(std::cell::RefCell::new(0.0));
            let on_progress = {
                let elapsed_total = elapsed_total.clone();
                std::rc::Rc::new(move |done: usize, total: usize, batch_ms: f64| {
                    set_load_progress.set((done, total));
                    *elapsed_total.borrow_mut() += batch_ms;
                    if done > 0 {
                        let per_op = *elapsed_total.borrow() / done as f64;
                        let remaining = (total - done) as f64 * per_op;
                        set_load_eta_ms.set(remaining as u64);
                    }
                })
            };

            let on_done = std::rc::Rc::new(move || {
                let txt = getEditorContent();
                if let Some(cb) = on_stats {
                    let lines = txt.lines().count();
                    let words = txt.split_whitespace().count();
                    cb.run(EditorStats {
                        chars: txt.len(),
                        words,
                        lines,
                    });
                }
                set_content.set(txt);
                set_playback_version.set(version);
                set_load_state.set("ready".to_string());
                set_load_progress.set((0, 0));
                set_load_eta_ms.set(0);
                leptos::logging::log!(
                    "Snapshot load complete: doc={}, elapsed_ms={}",
                    doc_id,
                    (now_ms() - load_start) as u64
                );
                ws.send(ClientMessage::RequestHistory { doc_id });
            });

            apply_ops_in_batches(
                delta_ops,
                PrefetchConfig {
                    target_ms: 8.0,
                    initial_batch: 16,
                    max_batch: 256,
                },
                apply_batch,
                on_progress,
                on_done,
            );
        }
        ServerMessage::History {
            doc_id: msg_doc_id,
            ops,
        } => {
            if msg_doc_id != doc_id {
                return;
            }
            leptos::logging::log!("Received History: {} ops", ops.len());
            set_history.set(ops);
        }
        ServerMessage::NewOp {
            doc_id: msg_doc_id,
            op,
            seq,
            client_id: origin_id,
        } => {
            if msg_doc_id != doc_id {
                return;
            }

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
                        cb.run(EditorStats {
                            chars: txt.len(),
                            words,
                            lines,
                        });
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
        ServerMessage::SyncHello {
            peer_id, vector: _, ..
        } => {
            leptos::logging::log!("P2P Handshake from Peer: {}", peer_id);
        }
        ServerMessage::Pong => {
            // leptos::logging::log!("Pong received");
        }
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

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

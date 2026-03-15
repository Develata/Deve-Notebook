use crate::api::{ConnectionStatus, WsService};
use crate::storage::identity::{note_handshake, save_repo_vector, sign_sync_hello};
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::super::types::HandshakeSignals;
use super::handshake_bootstrap::restore_session_scope;

/// 设置握手 Effect。
pub fn setup(ws: &WsService, signals: HandshakeSignals) {
    let ws_clone = ws.clone();
    let status_signal = ws.status;
    let endpoint_signal = ws.endpoint;
    let last_mode = Rc::new(RefCell::new(None::<String>));
    let scope_nonce = Rc::new(Cell::new(0u64));

    Effect::new(move |_| {
        let next_nonce = scope_nonce.get().saturating_add(1);
        scope_nonce.set(next_nonce);
        if status_signal.get() != ConnectionStatus::Connected {
            *last_mode.borrow_mut() = None;
            ws_clone.clear_writer_ready();
            signals.set_handshake_ready.set(false);
            signals.set_handshake_scope_nonce.set(None);
            return;
        }

        let ws = ws_clone.clone();
        let maybe_mode = signals.degraded.get();
        let maybe_identity = signals.identity.get();
        let active_repo_id = signals.current_repo_id.get();
        let vector = signals.repo_vector.get();
        let repo_name = signals.current_repo.get();
        let branch = signals.active_branch.get();
        let pending_branch_switch = signals.pending_branch_switch.get();
        let pending_repo_switch = signals.pending_repo_switch.get();
        let is_reconnect_bootstrap = last_mode.borrow().is_none();
        let should_restore = should_restore_session_scope(
            is_reconnect_bootstrap,
            pending_branch_switch.as_ref(),
            pending_repo_switch.as_deref(),
        );
        if should_suspend_handshake(
            &branch,
            pending_branch_switch.as_ref(),
            pending_repo_switch.as_deref(),
        ) {
            *last_mode.borrow_mut() = None;
            if should_restore {
                restore_session_scope(
                    &ws,
                    signals,
                    repo_name.clone(),
                    active_repo_id.clone(),
                    branch.clone(),
                );
            }
            ws.clear_writer_ready();
            signals.set_handshake_ready.set(false);
            signals.set_handshake_scope_nonce.set(None);
            return;
        }
        let Some(mode_key) = handshake_mode_key(
            &endpoint_signal.get(),
            maybe_mode.as_ref().map(|_| ()),
            maybe_identity.as_ref().map(|id| id.repo_id.as_str()),
            branch.as_ref(),
        ) else {
            return;
        };
        if last_mode.borrow().as_deref() == Some(mode_key.as_str()) {
            return;
        }
        *last_mode.borrow_mut() = Some(mode_key);
        ws_clone.clear_writer_ready();
        signals.set_handshake_ready.set(false);
        signals.set_handshake_scope_nonce.set(None);
        let scope_nonce = scope_nonce.clone();
        if let Some(identity) = maybe_identity.as_ref()
            && maybe_mode.is_none()
            && active_repo_id.as_deref() != Some(identity.repo_id.as_str())
        {
            *last_mode.borrow_mut() = None;
            if should_restore {
                restore_session_scope(
                    &ws,
                    signals,
                    repo_name.clone(),
                    active_repo_id.clone(),
                    branch.clone(),
                );
            }
            ws.clear_writer_ready();
            signals.set_handshake_ready.set(false);
            signals.set_handshake_scope_nonce.set(None);
            return;
        }
        signals.set_handshake_scope_nonce.set(Some(next_nonce));
        spawn_local(async move {
            if let Some(mode) = maybe_mode {
                leptos::logging::warn!("{}", mode.banner_text());
                if should_restore {
                    restore_session_scope(
                        &ws,
                        signals,
                        repo_name.clone(),
                        active_repo_id.clone(),
                        branch.clone(),
                    );
                }
                ws.clear_writer_ready();
                signals.set_handshake_ready.set(true);
                signals.set_handshake_scope_nonce.set(None);
                return;
            }
            let Some(identity) = maybe_identity else {
                return;
            };

            if should_restore {
                restore_session_scope(
                    &ws,
                    signals,
                    repo_name.clone(),
                    active_repo_id.clone(),
                    branch.clone(),
                );
            }

            leptos::logging::log!("已连接! 发送 SyncHello...");
            let sorted_map: BTreeMap<_, _> = vector.iter().collect();
            let vec_bytes = match serde_json::to_vec(&sorted_map) {
                Ok(bytes) => bytes,
                Err(err) => {
                    leptos::logging::error!("序列化握手向量失败: {}", err);
                    return;
                }
            };
            let mut msg = Vec::new();
            msg.extend_from_slice(b"deve-handshake");
            msg.extend_from_slice(identity.peer_id.as_bytes());
            msg.extend_from_slice(&vec_bytes);

            match sign_sync_hello(&identity, &msg).await {
                Ok(signature) => {
                    if scope_nonce.get() != next_nonce {
                        leptos::logging::warn!("忽略过期握手结果: scope 已变更");
                        return;
                    }
                    let peer_id = PeerId::new(&identity.peer_id);
                    match uuid::Uuid::parse_str(&identity.repo_id) {
                        Ok(repo_id) => {
                            match serde_json::to_string(&vector) {
                                Ok(vector_json) => {
                                    let _ = save_repo_vector(&identity.repo_id, &vector_json).await;
                                }
                                Err(err) => {
                                    leptos::logging::warn!("保存握手向量失败: {}", err);
                                }
                            }
                            let _ = note_handshake(&identity.repo_id).await;
                            let writer_peer_id = peer_id.clone();
                            ws.send(ClientMessage::SyncHello {
                                peer_id,
                                pub_key: identity.public_key.clone(),
                                signature,
                                vector,
                                repo_id,
                                scope_nonce: next_nonce,
                            });
                            ws.send(ClientMessage::RegisterWriter {
                                peer_id: writer_peer_id,
                                repo_id,
                                scope_nonce: next_nonce,
                            });
                        }
                        Err(err) => leptos::logging::error!(
                            "跳过 SyncHello: 非法 repo_id {} ({})",
                            identity.repo_id,
                            err
                        ),
                    }
                }
                Err(err) => leptos::logging::error!("WebCrypto 握手签名失败: {}", err),
            }
        });
    });
}

fn handshake_mode_key(
    endpoint: &str,
    degraded: Option<()>,
    repo_id: Option<&str>,
    branch: Option<&PeerId>,
) -> Option<String> {
    degraded
        .map(|_| format!("{endpoint}::degraded"))
        .or_else(|| {
            repo_id.map(|repo_id| {
                let branch_key = branch
                    .map(PeerId::to_string)
                    .unwrap_or_else(|| "local".to_string());
                format!("{endpoint}::{repo_id}::{branch_key}")
            })
        })
}

fn should_suspend_handshake(
    branch: &Option<PeerId>,
    pending_branch_switch: Option<&super::super::PendingBranchTarget>,
    pending_repo_switch: Option<&str>,
) -> bool {
    branch.is_some() || pending_branch_switch.is_some() || pending_repo_switch.is_some()
}

fn should_restore_session_scope(
    is_reconnect_bootstrap: bool,
    pending_branch_switch: Option<&super::super::PendingBranchTarget>,
    pending_repo_switch: Option<&str>,
) -> bool {
    is_reconnect_bootstrap && pending_branch_switch.is_none() && pending_repo_switch.is_none()
}

#[cfg(test)]
#[path = "handshake_test.rs"]
mod tests;

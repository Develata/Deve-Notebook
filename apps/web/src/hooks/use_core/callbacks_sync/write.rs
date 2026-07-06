//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::{
    WriteGateAction, WriteGateReason, cannot_send, reason_from_block,
};
use crate::i18n::Locale;
use crate::runtime::scope_client::{LocalScopeSignals, stable_local_scope_nonce};
use deve_core::models::DocId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

pub(super) struct SyncWriteCallbacks {
    pub(super) on_set_sync_mode: Callback<String>,
    pub(super) on_confirm_merge: Callback<()>,
    pub(super) on_discard_pending: Callback<()>,
    pub(super) on_merge_peer: Callback<String>,
}

pub(super) fn create_sync_write_callbacks(
    ws: &WsService,
    locale: RwSignal<Locale>,
    current_doc: ReadSignal<Option<DocId>>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> SyncWriteCallbacks {
    let ws1 = ws.clone();
    let on_set_sync_mode = Callback::new(move |mode: String| {
        let Some(scope_nonce) = sync_write_scope_nonce(
            &ws1,
            local_scope,
            write_gate,
            set_sync_banner,
            locale,
            WriteGateAction::SetSyncMode,
        ) else {
            return;
        };
        ws1.send(ClientMessage::SetSyncMode {
            mode,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws2 = ws.clone();
    let on_confirm_merge = Callback::new(move |_: ()| {
        let Some(scope_nonce) = sync_write_scope_nonce(
            &ws2,
            local_scope,
            write_gate,
            set_sync_banner,
            locale,
            WriteGateAction::ConfirmMerge,
        ) else {
            return;
        };
        ws2.send(ClientMessage::ConfirmMerge {
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws3 = ws.clone();
    let on_discard_pending = Callback::new(move |_: ()| {
        let Some(scope_nonce) = sync_write_scope_nonce(
            &ws3,
            local_scope,
            write_gate,
            set_sync_banner,
            locale,
            WriteGateAction::DiscardPending,
        ) else {
            return;
        };
        ws3.send(ClientMessage::DiscardPending {
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws4 = ws.clone();
    let on_merge_peer = Callback::new(move |peer_id: String| {
        let Some(scope_nonce) = sync_write_scope_nonce(
            &ws4,
            local_scope,
            write_gate,
            set_sync_banner,
            locale,
            WriteGateAction::MergePeer,
        ) else {
            return;
        };
        let Some(doc_id) = current_doc.get_untracked() else {
            let message = cannot_send(
                locale.get_untracked(),
                WriteGateAction::MergePeer,
                WriteGateReason::NoCurrentDocumentSelected,
            );
            warn_sync_banner(set_sync_banner, message);
            return;
        };
        ws4.send(ClientMessage::MergePeer {
            peer_id,
            doc_id,
            scope_nonce: Some(scope_nonce),
        });
    });

    SyncWriteCallbacks {
        on_set_sync_mode,
        on_confirm_merge,
        on_discard_pending,
        on_merge_peer,
    }
}

fn sync_write_scope_nonce(
    ws: &WsService,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
    action: WriteGateAction,
) -> Option<u64> {
    if let Some(block) = repo_write_block_untracked(ws, write_gate) {
        let message = cannot_send(locale.get_untracked(), action, reason_from_block(block));
        warn_sync_banner(set_sync_banner, message);
        return None;
    }
    let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
        let message = cannot_send(
            locale.get_untracked(),
            action,
            WriteGateReason::LocalRepoScopeUnstable,
        );
        warn_sync_banner(set_sync_banner, message);
        return None;
    };
    Some(scope_nonce)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ConnectionStatus;
    use crate::hooks::use_core::{LoadPhase, PendingBranchSwitch, PendingRepoSwitch};
    use crate::i18n::Locale;
    use deve_core::models::PeerId;
    use leptos::prelude::{GetUntracked, Signal, signal};

    #[test]
    fn merge_peer_sends_local_scope_when_writer_ready() {
        let owner = leptos::reactive::owner::Owner::new();
        owner.with(|| {
            let ws = ready_ws();
            let doc_id = DocId::new();
            let (current_doc, _) = signal(Some(doc_id));
            let (banner, set_banner) = signal(None::<String>);
            let locale = RwSignal::new(Locale::Zh);
            let callbacks = create_sync_write_callbacks(
                &ws,
                locale,
                current_doc,
                local_scope_signals(None),
                write_signals(None),
                set_banner,
            );

            callbacks.on_merge_peer.run("peer-a".into());

            let sent = ws.drain_sent_for_test();
            assert_eq!(banner.get_untracked(), None);
            match sent.as_slice() {
                [
                    ClientMessage::MergePeer {
                        peer_id,
                        doc_id: actual_doc_id,
                        scope_nonce,
                    },
                ] => {
                    assert_eq!(peer_id, "peer-a");
                    assert_eq!(*actual_doc_id, doc_id);
                    assert_eq!(*scope_nonce, Some(7));
                }
                other => panic!("expected one MergePeer message, got {other:?}"),
            }
        });
    }

    #[test]
    fn merge_peer_does_not_send_from_remote_branch_scope() {
        let owner = leptos::reactive::owner::Owner::new();
        owner.with(|| {
            let ws = ready_ws();
            let (current_doc, _) = signal(Some(DocId::new()));
            let (banner, set_banner) = signal(None::<String>);
            let active_branch = Some(PeerId::new("peer-a"));
            let locale = RwSignal::new(Locale::Zh);
            let callbacks = create_sync_write_callbacks(
                &ws,
                locale,
                current_doc,
                local_scope_signals(active_branch.clone()),
                write_signals(active_branch),
                set_banner,
            );

            callbacks.on_merge_peer.run("peer-a".into());

            assert!(ws.drain_sent_for_test().is_empty());
            assert_eq!(
                banner.get_untracked().as_deref(),
                Some("无法发送 合并节点请求：只读模式")
            );
        });
    }

    fn ready_ws() -> WsService {
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        ws.set_node_role_for_test("main");
        ws.mark_writer_ready("repo-1", 7, "browser-peer");
        ws
    }

    fn local_scope_signals(active_branch_value: Option<PeerId>) -> LocalScopeSignals {
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (current_scope_nonce, _) = signal(7u64);
        let (active_branch, _) = signal(active_branch_value);
        let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
        let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
        LocalScopeSignals {
            current_repo_id,
            current_scope_nonce,
            active_branch,
            pending_branch_switch,
            pending_repo_switch,
        }
    }

    fn write_signals(active_branch_value: Option<PeerId>) -> RepoWriteSignals {
        let (load_state, _) = signal(LoadPhase::Ready);
        let (handshake_ready, _) = signal(true);
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (current_scope_nonce, _) = signal(7u64);
        let (active_branch, _) = signal(active_branch_value);
        let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
        let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
        RepoWriteSignals {
            load_state,
            is_spectator: Signal::derive(|| false),
            handshake_ready,
            current_repo_id,
            current_scope_nonce,
            active_branch,
            pending_branch_switch,
            pending_repo_switch,
        }
    }
}

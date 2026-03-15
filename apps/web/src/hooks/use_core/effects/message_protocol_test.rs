use super::{ProtocolControlSignals, clear_failed_scope_switch};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::protocol::ServerErrorCode;
use leptos::prelude::*;

#[test]
fn switch_errors_clear_pending_scope_switches_only_for_matching_nonce() {
    for code in [
        ServerErrorCode::ScRepoContextInvalid,
        ServerErrorCode::ScRepoNotSelected,
        ServerErrorCode::SyncRepoUnbound,
    ] {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (pending_branch_switch, set_pending_branch_switch) =
            signal(Some(PendingBranchTarget::Local));
        let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(Some(7u64));
        let (pending_repo_switch, set_pending_repo_switch) = signal(Some("wiki".to_string()));
        let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(Some(7u64));
        let (shadow_list_request_id, set_shadow_list_request_id) =
            signal(Some("shadow-1".to_string()));
        let (repo_list_request_id, set_repo_list_request_id) = signal(Some("repo-1".to_string()));
        let (doc_list_request_id, set_doc_list_request_id) = signal(Some("doc-1".to_string()));
        let (tree_request_id, set_tree_request_id) = signal(Some("tree-1".to_string()));

        clear_failed_scope_switch(
            code,
            Some(7),
            ProtocolControlSignals {
                pending_branch_switch,
                pending_branch_switch_nonce,
                set_pending_branch_switch,
                set_pending_branch_switch_nonce,
                pending_repo_switch_nonce,
                set_pending_repo_switch,
                set_pending_repo_switch_nonce,
                set_shadow_list_request_id,
                set_repo_list_request_id,
                set_doc_list_request_id,
                set_tree_request_id,
            },
        );

        assert_eq!(pending_branch_switch.get_untracked(), None);
        assert_eq!(pending_repo_switch.get_untracked(), None);
        assert_eq!(shadow_list_request_id.get_untracked(), None);
        assert_eq!(repo_list_request_id.get_untracked(), None);
        assert_eq!(doc_list_request_id.get_untracked(), None);
        assert_eq!(tree_request_id.get_untracked(), None);
    }
}

#[test]
fn switch_errors_clear_orphan_repo_switch_nonce_without_pending_name() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (pending_branch_switch, set_pending_branch_switch) = signal(None);
    let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(None::<u64>);
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<String>);
    let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(Some(7u64));
    let (shadow_list_request_id, set_shadow_list_request_id) = signal(Some("shadow-1".to_string()));
    let (repo_list_request_id, set_repo_list_request_id) = signal(Some("repo-1".to_string()));
    let (doc_list_request_id, set_doc_list_request_id) = signal(Some("doc-1".to_string()));
    let (tree_request_id, set_tree_request_id) = signal(Some("tree-1".to_string()));

    clear_failed_scope_switch(
        ServerErrorCode::ScRepoContextInvalid,
        Some(7),
        ProtocolControlSignals {
            pending_branch_switch,
            pending_branch_switch_nonce,
            set_pending_branch_switch,
            set_pending_branch_switch_nonce,
            pending_repo_switch_nonce,
            set_pending_repo_switch,
            set_pending_repo_switch_nonce,
            set_shadow_list_request_id,
            set_repo_list_request_id,
            set_doc_list_request_id,
            set_tree_request_id,
        },
    );

    assert_eq!(pending_repo_switch.get_untracked(), None);
    assert_eq!(pending_repo_switch_nonce.get_untracked(), None);
    assert_eq!(shadow_list_request_id.get_untracked(), None);
    assert_eq!(repo_list_request_id.get_untracked(), None);
    assert_eq!(doc_list_request_id.get_untracked(), None);
    assert_eq!(tree_request_id.get_untracked(), None);
}

#[test]
fn stale_or_missing_nonce_keeps_pending_scope_switches() {
    for switch_nonce in [None, Some(9)] {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (pending_branch_switch, set_pending_branch_switch) =
            signal(Some(PendingBranchTarget::Local));
        let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(Some(7u64));
        let (pending_repo_switch, set_pending_repo_switch) = signal(Some("wiki".to_string()));
        let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(Some(7u64));
        let (_, set_shadow_list_request_id) = signal(Some("shadow-1".to_string()));
        let (_, set_repo_list_request_id) = signal(Some("repo-1".to_string()));
        let (_, set_doc_list_request_id) = signal(Some("doc-1".to_string()));
        let (_, set_tree_request_id) = signal(Some("tree-1".to_string()));

        clear_failed_scope_switch(
            ServerErrorCode::ScRepoContextInvalid,
            switch_nonce,
            ProtocolControlSignals {
                pending_branch_switch,
                pending_branch_switch_nonce,
                set_pending_branch_switch,
                set_pending_branch_switch_nonce,
                pending_repo_switch_nonce,
                set_pending_repo_switch,
                set_pending_repo_switch_nonce,
                set_shadow_list_request_id,
                set_repo_list_request_id,
                set_doc_list_request_id,
                set_tree_request_id,
            },
        );

        assert_eq!(
            pending_branch_switch.get_untracked(),
            Some(PendingBranchTarget::Local)
        );
        assert_eq!(
            pending_repo_switch.get_untracked(),
            Some("wiki".to_string())
        );
    }
}

#[test]
fn non_switch_errors_keep_pending_scope_switches() {
    for code in [
        ServerErrorCode::AuthTokenExpired,
        ServerErrorCode::RequestFailed,
        ServerErrorCode::StoragePersistFailed,
        ServerErrorCode::StorageDbLocked,
    ] {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (pending_branch_switch, set_pending_branch_switch) =
            signal(Some(PendingBranchTarget::Local));
        let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(Some(7u64));
        let (pending_repo_switch, set_pending_repo_switch) = signal(Some("wiki".to_string()));
        let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(Some(7u64));
        let (shadow_list_request_id, set_shadow_list_request_id) =
            signal(Some("shadow-1".to_string()));
        let (repo_list_request_id, set_repo_list_request_id) = signal(Some("repo-1".to_string()));
        let (doc_list_request_id, set_doc_list_request_id) = signal(Some("doc-1".to_string()));
        let (tree_request_id, set_tree_request_id) = signal(Some("tree-1".to_string()));

        clear_failed_scope_switch(
            code,
            Some(7),
            ProtocolControlSignals {
                pending_branch_switch,
                pending_branch_switch_nonce,
                set_pending_branch_switch,
                set_pending_branch_switch_nonce,
                pending_repo_switch_nonce,
                set_pending_repo_switch,
                set_pending_repo_switch_nonce,
                set_shadow_list_request_id,
                set_repo_list_request_id,
                set_doc_list_request_id,
                set_tree_request_id,
            },
        );

        assert_eq!(
            pending_branch_switch.get_untracked(),
            Some(PendingBranchTarget::Local)
        );
        assert_eq!(
            pending_repo_switch.get_untracked(),
            Some("wiki".to_string())
        );
        assert_eq!(
            shadow_list_request_id.get_untracked(),
            Some("shadow-1".to_string())
        );
        assert_eq!(
            repo_list_request_id.get_untracked(),
            Some("repo-1".to_string())
        );
        assert_eq!(
            doc_list_request_id.get_untracked(),
            Some("doc-1".to_string())
        );
        assert_eq!(tree_request_id.get_untracked(), Some("tree-1".to_string()));
    }
}

//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use super::{EditorDocumentTab, EditorTabKey, build_close_document_callback};
use crate::components::editor_tabs::diff_tab_from_session;
use crate::editor::EditorStats;
use crate::hooks::use_core::EditorContext;
use crate::hooks::use_core::navigation::PendingNavigation;
use crate::runtime::domain::{LoadPhase, PendingBranchSwitch, PendingRepoSwitch};
use crate::runtime::source_control_client::diff_session::DiffSessionWire;
use crate::runtime::{
    document::pending::PendingLocalEdits, document_client::DocumentClient,
    scope_client::ScopeClient, source_control_client::SourceControlClient,
};
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::RepoListEntry;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use deve_core::tree::FileNode;
use leptos::prelude::{Callable, Callback, GetUntracked, signal};

#[test]
fn closing_last_active_document_returns_home_without_activating_inactive_diff_tab() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let doc_id = DocId::from_u128(11);
    let inactive_diff = diff_tab_from_session(DiffSessionWire::new(
        "diff.md".into(),
        "old".into(),
        "new".into(),
    ));
    let inactive_diff_key = inactive_diff.key.clone();
    let (docs, _) = signal(vec![(doc_id, "notes/a.md".to_string())]);
    let (current_doc, set_current_doc) = signal(Some(doc_id));
    let (explicit_home, set_explicit_home) = signal(false);
    let (pending_local_edits, set_pending_local_edits) = signal(PendingLocalEdits::new());
    let (tree_nodes, _) = signal(Vec::<FileNode>::new());
    let document = DocumentClient {
        docs,
        current_doc,
        set_current_doc,
        set_explicit_home,
        pending_local_edits,
        set_pending_local_edits,
        on_doc_select: noop(),
        on_doc_create: noop(),
        on_doc_rename: noop(),
        on_doc_delete: noop(),
        on_doc_copy: noop(),
        on_doc_move: noop(),
        tree_nodes,
    };
    let (pending_navigation, set_pending_navigation) = signal(None::<PendingNavigation>);
    let (stats, _) = signal(EditorStats {
        chars: 0,
        words: 0,
        lines: 0,
    });
    let (load_state, set_load_state) = signal(LoadPhase::Ready);
    let (load_progress, set_load_progress) = signal((0usize, 0usize));
    let (load_eta_ms, set_load_eta_ms) = signal(0u64);
    let (doc_version, set_doc_version) = signal(0u64);
    let (playback_version, set_playback_version) = signal(0u64);
    let (active_branch, set_active_branch) = signal(None::<PeerId>);
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (current_repo_id, set_current_repo_id) = signal(None::<String>);
    let (current_scope_nonce, _) = signal(0u64);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    let (handshake_ready, _) = signal(false);
    let (handshake_scope_nonce, _) = signal(None::<u64>);
    let (is_spectator, _) = signal(false);
    let editor = EditorContext {
        docs,
        current_doc,
        stats,
        on_stats: noop(),
        load_state,
        set_load_state,
        load_progress,
        set_load_progress,
        load_eta_ms,
        set_load_eta_ms,
        doc_version,
        set_doc_version,
        playback_version,
        set_playback_version,
        is_spectator: is_spectator.into(),
        active_branch,
        pending_branch_switch,
        current_repo_id,
        current_scope_nonce,
        pending_repo_switch,
        handshake_ready,
        handshake_scope_nonce,
        pending_local_edits,
        set_pending_local_edits,
        set_pending_navigation,
    };
    let (current_repo, set_current_repo) = signal(None::<String>);
    let (shadow_repos, _) = signal(Vec::<String>::new());
    let (repo_list, _) = signal(Vec::<String>::new());
    let (repo_entries, _) = signal(Vec::<RepoListEntry>::new());
    let scope = ScopeClient {
        current_doc,
        current_repo,
        current_repo_id,
        current_scope_nonce,
        active_branch,
        set_active_branch,
        pending_repo_switch,
        on_switch_repo: noop(),
        on_create_repo: noop(),
        on_rename_repo: noop(),
        on_remove_repo: noop(),
        on_switch_branch: noop(),
        set_current_repo,
        set_current_repo_id,
        shadow_repos,
        on_list_shadows: noop(),
        repo_list,
        repo_entries,
        is_spectator: is_spectator.into(),
    };
    let (diff_content, set_diff_content) = signal(None::<DiffSessionWire>);
    let (commit_diff_request_id, set_commit_diff_request_id) = signal(None::<String>);
    let (commit_diff_result, set_commit_diff_result) = signal(Vec::<CommitFileDiff>::new());
    let source_control = SourceControlClient {
        staged_changes: signal(Vec::<ChangeEntry>::new()).0,
        unstaged_changes: signal(Vec::<ChangeEntry>::new()).0,
        confirmed_changes: signal(Vec::<ChangeEntry>::new()).0,
        commit_history: signal(Vec::<CommitInfo>::new()).0,
        commit_history_request_id: signal(None::<String>).0,
        commit_diff_request_id,
        set_commit_diff_request_id,
        on_get_changes: noop(),
        on_stage_file: noop(),
        on_stage_files: noop(),
        on_unstage_file: noop(),
        on_unstage_files: noop(),
        on_discard_file: noop(),
        on_commit: noop(),
        on_get_history: noop(),
        diff_content,
        set_diff_content,
        on_get_doc_diff: noop(),
        commit_diff_result,
        set_commit_diff_result,
        on_resolve_conflict: noop(),
        on_get_commit_diff: noop(),
        on_commit_and_push: noop(),
    };
    let (doc_tabs, set_doc_tabs) = signal(vec![EditorDocumentTab {
        doc_id,
        title: "a.md".into(),
        tooltip: "notes/a.md".into(),
    }]);
    let (diff_tabs, _) = signal(vec![inactive_diff]);
    let (tab_order, set_tab_order) = signal(vec![
        EditorTabKey::Document(doc_id),
        EditorTabKey::Diff(inactive_diff_key.clone()),
    ]);
    let (doc_access_order, set_doc_access_order) = signal(vec![doc_id]);
    let close_document = build_close_document_callback(
        &document,
        &editor,
        &scope,
        &source_control,
        doc_tabs,
        set_doc_tabs,
        tab_order,
        set_tab_order,
        doc_access_order,
        set_doc_access_order,
    );

    close_document.run(doc_id);

    assert!(doc_tabs.get_untracked().is_empty());
    assert_eq!(
        tab_order.get_untracked(),
        vec![EditorTabKey::Diff(inactive_diff_key)]
    );
    assert!(doc_access_order.get_untracked().is_empty());
    assert_eq!(current_doc.get_untracked(), None);
    assert!(explicit_home.get_untracked());
    assert!(diff_content.get_untracked().is_none());
    assert!(pending_navigation.get_untracked().is_none());
    assert_eq!(diff_tabs.get_untracked().len(), 1);
}

fn noop<T>() -> Callback<T>
where
    T: 'static,
{
    Callback::new(|_| {})
}

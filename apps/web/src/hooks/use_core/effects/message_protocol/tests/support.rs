use super::super::ProtocolControlSignals;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use leptos::prelude::{GetUntracked, ReadSignal, signal};

pub struct ProtocolSignalHarness {
    _runtime: leptos::reactive::owner::Owner,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub pending_repo_switch_nonce: ReadSignal<Option<u64>>,
    shadow_list_request_id: ReadSignal<Option<String>>,
    repo_list_request_id: ReadSignal<Option<String>>,
    doc_list_request_id: ReadSignal<Option<String>>,
    tree_request_id: ReadSignal<Option<String>>,
    sync_mode_request_id: ReadSignal<Option<String>>,
    pending_ops_request_id: ReadSignal<Option<String>>,
    pub search_request_id: ReadSignal<Option<String>>,
    pub search_results: ReadSignal<Vec<(String, String, f32)>>,
    changes_request_id: ReadSignal<Option<String>>,
    commit_history_request_id: ReadSignal<Option<String>>,
    doc_diff_request_id: ReadSignal<Option<String>>,
    commit_diff_request_id: ReadSignal<Option<String>>,
    pub source_control_notice: ReadSignal<Option<SourceControlNotice>>,
    pub sync_banner: ReadSignal<Option<String>>,
    control: ProtocolControlSignals,
}

pub fn protocol_signal_harness(
    pending_branch: Option<PendingBranchTarget>,
    pending_branch_nonce: Option<u64>,
    pending_repo: Option<&str>,
    pending_repo_nonce: Option<u64>,
) -> ProtocolSignalHarness {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (pending_branch_switch, set_pending_branch_switch) = signal(pending_branch);
    let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) =
        signal(pending_branch_nonce);
    let (pending_repo_switch, set_pending_repo_switch) = signal(pending_repo.map(str::to_string));
    let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(pending_repo_nonce);
    let (shadow_list_request_id, set_shadow_list_request_id) = signal(Some("shadow-1".to_string()));
    let (repo_list_request_id, set_repo_list_request_id) = signal(Some("repo-1".to_string()));
    let (doc_list_request_id, set_doc_list_request_id) = signal(Some("doc-1".to_string()));
    let (tree_request_id, set_tree_request_id) = signal(Some("tree-1".to_string()));
    let (sync_mode_request_id, set_sync_mode_request_id) = signal(Some("sync-1".to_string()));
    let (pending_ops_request_id, set_pending_ops_request_id) =
        signal(Some("pending-1".to_string()));
    let (search_request_id, set_search_request_id) = signal(Some("search-1".to_string()));
    let (search_results, set_search_results) =
        signal(vec![("doc-1".to_string(), "notes/a.md".to_string(), 1.0)]);
    let (changes_request_id, set_changes_request_id) = signal(Some("changes-1".to_string()));
    let (commit_history_request_id, set_commit_history_request_id) =
        signal(Some("history-1".to_string()));
    let (doc_diff_request_id, set_doc_diff_request_id) = signal(Some("diff-1".to_string()));
    let (commit_diff_request_id, set_commit_diff_request_id) =
        signal(Some("commit-diff-1".to_string()));
    let (source_control_notice, set_source_control_notice) = signal(None::<SourceControlNotice>);
    let (sync_banner, set_sync_banner) = signal(None::<String>);

    ProtocolSignalHarness {
        _runtime: runtime,
        pending_branch_switch,
        pending_repo_switch,
        pending_repo_switch_nonce,
        shadow_list_request_id,
        repo_list_request_id,
        doc_list_request_id,
        tree_request_id,
        sync_mode_request_id,
        pending_ops_request_id,
        search_request_id,
        search_results,
        changes_request_id,
        commit_history_request_id,
        doc_diff_request_id,
        commit_diff_request_id,
        source_control_notice,
        sync_banner,
        control: ProtocolControlSignals {
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
            set_sync_mode_request_id,
            set_pending_ops_request_id,
            search_request_id,
            set_search_request_id,
            set_search_results,
            changes_request_id,
            set_changes_request_id,
            commit_history_request_id,
            set_commit_history_request_id,
            doc_diff_request_id,
            set_doc_diff_request_id,
            commit_diff_request_id,
            set_commit_diff_request_id,
            set_source_control_notice,
            set_sync_banner,
        },
    }
}

impl ProtocolSignalHarness {
    pub fn control(&self) -> ProtocolControlSignals {
        self.control
    }

    pub fn assert_all_requests_cleared(&self) {
        assert_eq!(self.shadow_list_request_id.get_untracked(), None);
        assert_eq!(self.repo_list_request_id.get_untracked(), None);
        assert_eq!(self.doc_list_request_id.get_untracked(), None);
        assert_eq!(self.tree_request_id.get_untracked(), None);
        assert_eq!(self.sync_mode_request_id.get_untracked(), None);
        assert_eq!(self.pending_ops_request_id.get_untracked(), None);
        assert_eq!(self.search_request_id.get_untracked(), None);
        assert_eq!(self.changes_request_id.get_untracked(), None);
        assert_eq!(self.commit_history_request_id.get_untracked(), None);
        assert_eq!(self.doc_diff_request_id.get_untracked(), None);
        assert_eq!(self.commit_diff_request_id.get_untracked(), None);
    }

    pub fn assert_source_control_requests_cleared(&self) {
        assert_eq!(self.changes_request_id.get_untracked(), None);
        assert_eq!(self.commit_history_request_id.get_untracked(), None);
        assert_eq!(self.doc_diff_request_id.get_untracked(), None);
        assert_eq!(self.commit_diff_request_id.get_untracked(), None);
    }

    pub fn assert_search_request_cleared(&self) {
        assert_eq!(self.search_request_id.get_untracked(), None);
        assert!(self.search_results.get_untracked().is_empty());
    }
}

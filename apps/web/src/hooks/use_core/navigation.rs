//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime
//!
use deve_core::models::DocId;
use leptos::prelude::*;

use super::pending::{PendingLocalEdits, PendingScope, has_pending_edits_for_doc_in_scope};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationTarget {
    Doc,
    Repo,
    Branch,
    Home,
}

#[derive(Clone)]
pub struct PendingNavigation {
    pub target: NavigationTarget,
    pub action: Callback<()>,
}

pub fn guard_navigation(
    current_doc: Option<DocId>,
    current_repo_id: Option<&str>,
    current_scope_nonce: u64,
    pending_local_edits: &PendingLocalEdits,
    set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    target: NavigationTarget,
    action: Callback<()>,
) -> bool {
    if has_pending_for_current_doc(
        current_doc,
        current_repo_id,
        current_scope_nonce,
        pending_local_edits,
    ) {
        set_pending_navigation.set(Some(PendingNavigation { target, action }));
        return false;
    }
    action.run(());
    true
}

fn has_pending_for_current_doc(
    current_doc: Option<DocId>,
    current_repo_id: Option<&str>,
    current_scope_nonce: u64,
    pending_local_edits: &PendingLocalEdits,
) -> bool {
    let Some(scope) = PendingScope::from_repo_id_str(current_repo_id, current_scope_nonce) else {
        return false;
    };
    current_doc.is_some_and(|doc_id| {
        has_pending_edits_for_doc_in_scope(pending_local_edits, doc_id, scope)
    })
}

#[cfg(test)]
mod tests {
    use super::{NavigationTarget, guard_navigation};
    use crate::hooks::use_core::pending::{
        PendingLocalEditInput, PendingLocalEdits, push_pending_edit,
    };
    use deve_core::models::{DocId, Op, RepoId};
    use leptos::prelude::{Callback, GetUntracked, signal};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn pending_input(repo_id: RepoId, doc_id: DocId, scope_nonce: u64) -> PendingLocalEditInput {
        PendingLocalEditInput {
            repo_id,
            doc_id,
            scope_nonce,
            client_id: 11,
            client_op_id: scope_nonce,
            base_version: 0,
            op: Op::Insert {
                pos: 0,
                content: "pending".into(),
            },
        }
    }

    #[test]
    fn navigation_guard_only_blocks_current_pending_scope() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let repo_id = RepoId::new_v4();
        let doc_id = DocId::from_u128(71);
        let mut pending = PendingLocalEdits::new();
        push_pending_edit(&mut pending, pending_input(repo_id, doc_id, 6));
        let (pending_navigation, set_pending_navigation) = signal(None);
        let action_ran = Arc::new(AtomicBool::new(false));
        let action_ran_for_callback = action_ran.clone();

        assert!(guard_navigation(
            Some(doc_id),
            Some(&repo_id.to_string()),
            7,
            &pending,
            set_pending_navigation,
            NavigationTarget::Doc,
            Callback::new(move |_| action_ran_for_callback.store(true, Ordering::Relaxed)),
        ));
        assert!(action_ran.load(Ordering::Relaxed));
        assert!(pending_navigation.get_untracked().is_none());
    }

    #[test]
    fn navigation_guard_blocks_current_scope_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let repo_id = RepoId::new_v4();
        let doc_id = DocId::from_u128(72);
        let mut pending = PendingLocalEdits::new();
        push_pending_edit(&mut pending, pending_input(repo_id, doc_id, 7));
        let (pending_navigation, set_pending_navigation) = signal(None);
        let action_ran = Arc::new(AtomicBool::new(false));
        let action_ran_for_callback = action_ran.clone();

        assert!(!guard_navigation(
            Some(doc_id),
            Some(&repo_id.to_string()),
            7,
            &pending,
            set_pending_navigation,
            NavigationTarget::Doc,
            Callback::new(move |_| action_ran_for_callback.store(true, Ordering::Relaxed)),
        ));
        assert!(!action_ran.load(Ordering::Relaxed));
        assert!(pending_navigation.get_untracked().is_some());
    }
}

//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime
//!
use deve_core::models::DocId;
use leptos::prelude::*;

use super::pending::{PendingLocalEdits, has_pending_edits_for_doc};

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
    pending_local_edits: &PendingLocalEdits,
    set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    target: NavigationTarget,
    action: Callback<()>,
) -> bool {
    if has_pending_for_current_doc(current_doc, pending_local_edits) {
        set_pending_navigation.set(Some(PendingNavigation { target, action }));
        return false;
    }
    action.run(());
    true
}

fn has_pending_for_current_doc(
    current_doc: Option<DocId>,
    pending_local_edits: &PendingLocalEdits,
) -> bool {
    current_doc.is_some_and(|doc_id| has_pending_edits_for_doc(pending_local_edits, doc_id))
}

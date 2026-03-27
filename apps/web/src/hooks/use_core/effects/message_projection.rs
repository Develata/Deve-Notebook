use crate::hooks::use_core::apply::apply_tree_delta;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::PeerId;
use deve_core::models::RepoId;
use deve_core::tree::TreeDelta;
use leptos::prelude::{GetUntracked, Set, Update};

use super::message_repo_scope::{matches_current_message_scope, matches_projection_message_scope};
use super::message_scope::accepts_system_or_matching_request;

pub fn handle_doc_list(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    docs: Vec<(deve_core::models::DocId, String)>,
    signals: CoreSignals,
) {
    let matches_scope = matches_current_message_scope(&repo_id, &branch, signals);
    let matches_projection_scope = matches_projection_message_scope(&repo_id, &branch, signals)
        && scope_nonce == Some(signals.current_scope_nonce.get_untracked());
    let matches_request = accepts_system_or_matching_request(
        request_id.as_deref(),
        signals.doc_list_request_id.get_untracked().as_deref(),
        scope_nonce,
        signals.current_scope_nonce.get_untracked(),
    );
    if !(matches_scope || matches_projection_scope) || !matches_request {
        leptos::logging::warn!("忽略 DocList: repo-scope 或 request gate 不匹配");
        return;
    }
    signals.set_doc_list_request_id.set(None);
    if request_id.is_none() {
        signals.set_tree_request_id.set(None);
    }
    if let Some(selected) = signals.current_doc.get_untracked()
        && !docs.iter().any(|(doc_id, _)| *doc_id == selected)
    {
        leptos::logging::log!("清理过期 current_doc: {} 不在当前 DocList 中", selected);
        signals.set_current_doc.set(None);
    }
    if signals.current_doc.get_untracked().is_none()
        && let Some(pending_path) = signals.pending_created_doc_path.get_untracked()
        && let Some((doc_id, _)) = docs.iter().find(|(_, path)| *path == pending_path)
    {
        signals.set_current_doc.set(Some(*doc_id));
        signals.set_pending_created_doc_path.set(None);
    }
    signals.set_docs.set(docs);
}

pub fn handle_tree_update(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    delta: TreeDelta,
    signals: CoreSignals,
) {
    let matches_scope = matches_current_message_scope(&repo_id, &branch, signals);
    let matches_projection_scope = matches_projection_message_scope(&repo_id, &branch, signals)
        && scope_nonce == Some(signals.current_scope_nonce.get_untracked());
    let matches_request = accepts_system_or_matching_request(
        request_id.as_deref(),
        signals.tree_request_id.get_untracked().as_deref(),
        scope_nonce,
        signals.current_scope_nonce.get_untracked(),
    );
    if !(matches_scope || matches_projection_scope) || !matches_request {
        leptos::logging::warn!("忽略 TreeUpdate: repo-scope 或 request gate 不匹配");
        return;
    }
    signals.set_tree_request_id.set(None);
    if request_id.is_none() {
        signals.set_doc_list_request_id.set(None);
    }
    signals
        .set_tree_nodes
        .update(|nodes| apply_tree_delta(nodes, delta));
}

#[cfg(test)]
mod tests {
    use super::handle_doc_list;
    use crate::hooks::use_core::state_init::init_signals;
    use deve_core::models::DocId;
    use leptos::prelude::*;

    #[test]
    fn doc_list_clears_stale_current_doc() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Disconnected).0);
        let stale = DocId::new();
        let fresh = DocId::new();
        signals.set_current_doc.set(Some(stale));

        handle_doc_list(
            None,
            None,
            None,
            Some(0),
            vec![(fresh, "notes/fresh.md".into())],
            signals,
        );

        assert_eq!(signals.current_doc.get_untracked(), None);
    }

    #[test]
    fn doc_list_preserves_current_doc_when_it_still_exists() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Disconnected).0);
        let selected = DocId::new();
        signals.set_current_doc.set(Some(selected));

        handle_doc_list(
            None,
            None,
            None,
            Some(0),
            vec![(selected, "notes/selected.md".into())],
            signals,
        );

        assert_eq!(signals.current_doc.get_untracked(), Some(selected));
    }

    #[test]
    fn doc_list_selects_pending_created_doc_when_no_doc_is_open() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Disconnected).0);
        let created = DocId::new();
        signals
            .set_pending_created_doc_path
            .set(Some("Untitled.md".to_string()));

        handle_doc_list(
            None,
            None,
            None,
            Some(0),
            vec![(created, "Untitled.md".into())],
            signals,
        );

        assert_eq!(signals.current_doc.get_untracked(), Some(created));
        assert_eq!(signals.pending_created_doc_path.get_untracked(), None);
    }
}

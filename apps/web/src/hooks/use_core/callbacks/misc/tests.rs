use super::{MiscCallbacks, MiscRequestSignals, SearchScopeSignals, create_misc_callbacks};
use crate::api::{ConnectionStatus, WsService};
use crate::editor::EditorStats;
use crate::hooks::use_core::{
    LoadPhase, PendingBranchSwitch, PendingBranchTarget, PendingRepoSwitch, SearchHit,
};
use deve_core::protocol::ClientMessage;
use leptos::prelude::{Callable, GetUntracked, ReadSignal, signal};
use leptos::reactive::owner::Owner;

struct SearchHarness {
    _runtime: Owner,
    ws: WsService,
    sync_banner: ReadSignal<Option<String>>,
    search_request_id: ReadSignal<Option<String>>,
    search_results: ReadSignal<Vec<SearchHit>>,
    callbacks: MiscCallbacks,
}

fn search_harness(
    load_state_value: &str,
    pending_branch_value: Option<PendingBranchTarget>,
    pending_repo_value: Option<String>,
) -> SearchHarness {
    let runtime = Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let (_stats, set_stats) = signal(EditorStats::default());
    let (load_state, _) = signal(LoadPhase::from_wire_or_ready(load_state_value));
    let (current_scope_nonce, _) = signal(37u64);
    let (pending_branch_switch, _) =
        signal(pending_branch_value.map(|target| PendingBranchSwitch::new(target, 1)));
    let (pending_repo_switch, _) =
        signal(pending_repo_value.map(|name| PendingRepoSwitch::switch(name, 1)));
    let (search_request_id, set_search_request_id) = signal(Some("previous-search".to_string()));
    let (search_results, set_search_results) = signal(vec![SearchHit::new(
        "doc.md".to_string(),
        "existing".to_string(),
        1.0,
    )]);
    let (_plugin_request_ids, set_plugin_request_ids) = signal(Vec::<String>::new());
    let (sync_banner, set_sync_banner) = signal(None::<String>);

    let callbacks = create_misc_callbacks(
        &ws,
        set_stats,
        load_state,
        SearchScopeSignals {
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
        },
        MiscRequestSignals {
            set_plugin_request_ids,
            set_search_request_id,
            set_search_results,
        },
        set_sync_banner,
    );

    SearchHarness {
        _runtime: runtime,
        ws,
        sync_banner,
        search_request_id,
        search_results,
        callbacks,
    }
}

#[test]
fn large_doc_search_gate_blocks_until_prefetch_ready() {
    for load_state in ["loading", "partial"] {
        let harness = search_harness(load_state, None, None);

        harness.callbacks.on_search.run("needle".to_string());

        assert!(harness.ws.drain_sent_for_test().is_empty());
        assert_eq!(
            harness.search_request_id.get_untracked().as_deref(),
            Some("previous-search")
        );
        assert_eq!(harness.search_results.get_untracked().len(), 1);
        assert_eq!(
            harness.sync_banner.get_untracked().as_deref(),
            Some("Cannot search: snapshot loading")
        );
    }
}

#[test]
fn large_doc_search_gate_sends_after_ready() {
    let harness = search_harness("ready", None, None);

    harness.callbacks.on_search.run("needle".to_string());

    let request_id = harness
        .search_request_id
        .get_untracked()
        .expect("search request id");
    assert_ne!(request_id, "previous-search");
    assert!(harness.search_results.get_untracked().is_empty());
    assert!(harness.sync_banner.get_untracked().is_none());
    let sent = harness.ws.drain_sent_for_test();
    assert_eq!(sent.len(), 1);
    match &sent[0] {
        ClientMessage::Search {
            request_id: sent_request_id,
            query,
            limit,
            scope_nonce,
        } => {
            assert_eq!(sent_request_id, &request_id);
            assert_eq!(query, "needle");
            assert_eq!(*limit, 50);
            assert_eq!(*scope_nonce, Some(37));
        }
        other => panic!("expected Search, got {other:?}"),
    }
}

#[test]
fn large_doc_search_gate_blocks_during_scope_switch() {
    let harness = search_harness("ready", Some(PendingBranchTarget::Local), None);

    harness.callbacks.on_search.run("needle".to_string());

    assert!(harness.ws.drain_sent_for_test().is_empty());
    assert_eq!(
        harness.search_request_id.get_untracked().as_deref(),
        Some("previous-search")
    );
    assert_eq!(
        harness.sync_banner.get_untracked().as_deref(),
        Some("Cannot search: scope switching")
    );
}

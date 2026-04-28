#[cfg(feature = "search")]
mod feature_enabled {
    use super::super::{classify_search_error, handle_search, score_match, search_scope_documents};
    use crate::server::AppState;
    use crate::server::channel::DualChannel;
    use crate::server::edit_state_test_support::{edit_harness, seed_doc_with_content};
    use crate::server::repo_scope::ResolvedRepo;
    use crate::server::session::WsSession;
    use deve_core::protocol::{ServerErrorCode, ServerMessage};
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[test]
    fn classifies_search_index_corruption_as_storage_persist_failed() {
        assert_eq!(
            classify_search_error("Search index document missing required stored field: path"),
            ServerErrorCode::StoragePersistFailed
        );
        assert_eq!(
            classify_search_error("SearchService writer lock poisoned"),
            ServerErrorCode::StoragePersistFailed
        );
    }

    #[test]
    fn keeps_user_query_errors_as_request_failed() {
        assert_eq!(
            classify_search_error("The query parser expected a term"),
            ServerErrorCode::RequestFailed
        );
    }

    #[test]
    fn scores_path_matches_above_content_matches() {
        assert!(score_match("notes/rust.md", "plain text", "rust").unwrap() > 1.0);
        assert_eq!(score_match("notes/a.md", "Rust content", "rust"), Some(1.0));
        assert_eq!(score_match("notes/a.md", "plain text", "rust"), None);
    }

    #[test]
    fn scope_search_scans_current_repo_documents() -> anyhow::Result<()> {
        let h = edit_harness(true)?;
        let rust_doc = seed_doc_with_content(&h.state, "default", "notes/rust.md", "Rust search")?;
        seed_doc_with_content(&h.state, "test", "notes/rust.md", "Other repo Rust")?;
        let scope = ResolvedRepo {
            repo_id: h.default_repo_id,
            repo_name: "default".into(),
            branch: None,
        };

        let results = search_scope_documents(&h.state, &scope, "rust", 10)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, rust_doc.to_string());
        assert_eq!(results[0].1, "notes/rust.md");
        Ok(())
    }

    #[test]
    fn scope_search_honors_limit_and_blank_query() -> anyhow::Result<()> {
        let h = edit_harness(false)?;
        seed_doc_with_content(&h.state, "default", "notes/a.md", "needle")?;
        seed_doc_with_content(&h.state, "default", "notes/b.md", "needle")?;
        let scope = ResolvedRepo {
            repo_id: h.default_repo_id,
            repo_name: "default".into(),
            branch: None,
        };

        assert!(search_scope_documents(&h.state, &scope, "   ", 10)?.is_empty());
        assert_eq!(
            search_scope_documents(&h.state, &scope, "needle", 1)?.len(),
            1
        );
        Ok(())
    }

    #[test]
    fn scope_search_orders_by_score_then_path_before_limit() -> anyhow::Result<()> {
        let h = edit_harness(false)?;
        let path_match =
            seed_doc_with_content(&h.state, "default", "notes/needle-alpha.md", "plain")?;
        seed_doc_with_content(&h.state, "default", "notes/b.md", "needle")?;
        seed_doc_with_content(&h.state, "default", "notes/a.md", "needle")?;
        let scope = ResolvedRepo {
            repo_id: h.default_repo_id,
            repo_name: "default".into(),
            branch: None,
        };

        let results = search_scope_documents(&h.state, &scope, "needle", 3)?;
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, path_match.to_string());
        assert_eq!(results[0].1, "notes/needle-alpha.md");
        assert_eq!(results[1].1, "notes/a.md");
        assert_eq!(results[2].1, "notes/b.md");

        let limited = search_scope_documents(&h.state, &scope, "needle", 1)?;
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].0, path_match.to_string());
        Ok(())
    }

    #[tokio::test]
    async fn handler_returns_structured_error_when_runtime_search_is_disabled() -> anyhow::Result<()>
    {
        let h = edit_harness(false)?;
        let (ch, mut rx) = test_channel(&h.state);
        let mut session = session_for_repo("default", h.default_repo_id);

        handle_search(
            &h.state,
            &ch,
            &mut session,
            "search-1".into(),
            "needle".into(),
            10,
            Some(44),
        )
        .await;

        match rx.recv().await {
            Some(ServerMessage::ProtocolError {
                error, scope_nonce, ..
            }) => {
                assert_eq!(error.code, ServerErrorCode::RequestFailed);
                assert_eq!(
                    error.detail.as_deref(),
                    Some("Search feature disabled for current runtime profile")
                );
                assert_eq!(scope_nonce, Some(44));
            }
            other => panic!("expected ProtocolError, got {:?}", other),
        }
        Ok(())
    }

    #[tokio::test]
    async fn handler_emits_repo_scoped_search_results() -> anyhow::Result<()> {
        let h = edit_harness(false)?;
        let doc_id = seed_doc_with_content(&h.state, "default", "notes/search.md", "Needle body")?;
        let state = search_enabled_state(&h.state);
        let (ch, mut rx) = test_channel(&state);
        let mut session = session_for_repo("default", h.default_repo_id);

        handle_search(
            &state,
            &ch,
            &mut session,
            "search-1".into(),
            "needle".into(),
            10,
            Some(44),
        )
        .await;

        match rx.recv().await {
            Some(ServerMessage::SearchResults {
                request_id,
                repo_id,
                branch,
                scope_nonce,
                results,
            }) => {
                assert_eq!(request_id, "search-1");
                assert_eq!(repo_id, Some(h.default_repo_id));
                assert_eq!(branch, None);
                assert_eq!(scope_nonce, Some(44));
                assert_eq!(
                    results,
                    vec![(doc_id.to_string(), "notes/search.md".into(), 1.0)]
                );
            }
            other => panic!("expected SearchResults, got {:?}", other),
        }
        Ok(())
    }

    #[tokio::test]
    async fn handler_returns_scoped_empty_results_for_blank_query_and_zero_limit()
    -> anyhow::Result<()> {
        let h = edit_harness(false)?;
        seed_doc_with_content(&h.state, "default", "notes/search.md", "Needle body")?;
        let state = search_enabled_state(&h.state);
        let (ch, mut rx) = test_channel(&state);
        let mut session = session_for_repo("default", h.default_repo_id);

        handle_search(
            &state,
            &ch,
            &mut session,
            "blank-query".into(),
            "   ".into(),
            10,
            Some(44),
        )
        .await;
        assert_scoped_empty_results(&mut rx, "blank-query", h.default_repo_id, Some(44)).await;

        handle_search(
            &state,
            &ch,
            &mut session,
            "zero-limit".into(),
            "needle".into(),
            0,
            Some(45),
        )
        .await;
        assert_scoped_empty_results(&mut rx, "zero-limit", h.default_repo_id, Some(45)).await;
        Ok(())
    }

    fn session_for_repo(repo_name: &str, repo_id: uuid::Uuid) -> WsSession {
        let mut session = WsSession::new();
        session.switch_repo(repo_name.into(), Some(repo_id));
        session
    }

    fn test_channel(state: &Arc<AppState>) -> (DualChannel, mpsc::Receiver<ServerMessage>) {
        let (tx, rx) = mpsc::channel(8);
        (DualChannel::new(state.tx.clone(), tx), rx)
    }

    async fn assert_scoped_empty_results(
        rx: &mut mpsc::Receiver<ServerMessage>,
        expected_request_id: &str,
        expected_repo_id: uuid::Uuid,
        expected_scope_nonce: Option<u64>,
    ) {
        match rx.recv().await {
            Some(ServerMessage::SearchResults {
                request_id,
                repo_id,
                branch,
                scope_nonce,
                results,
            }) => {
                assert_eq!(request_id, expected_request_id);
                assert_eq!(repo_id, Some(expected_repo_id));
                assert_eq!(branch, None);
                assert_eq!(scope_nonce, expected_scope_nonce);
                assert!(results.is_empty());
            }
            other => panic!("expected empty SearchResults, got {:?}", other),
        }
    }

    fn search_enabled_state(source: &Arc<AppState>) -> Arc<AppState> {
        Arc::new(AppState {
            repo: source.repo.clone(),
            sync_manager: source.sync_manager.clone(),
            tx: source.tx.clone(),
            plugins: Vec::new(),
            sync_engine: source.sync_engine.clone(),
            tree_manager: source.tree_manager.clone(),
            search_available: true,
            identity_key: source.identity_key.clone(),
        })
    }
}

#[cfg(not(feature = "search"))]
mod feature_disabled {
    use super::super::handle_search;
    use crate::server::channel::DualChannel;
    use crate::server::edit_state_test_support::edit_harness;
    use crate::server::session::WsSession;
    use deve_core::protocol::{ServerErrorCode, ServerMessage};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn handler_returns_structured_error_when_search_feature_is_not_compiled()
    -> anyhow::Result<()> {
        let h = edit_harness(false)?;
        let (tx, mut rx) = mpsc::channel(8);
        let ch = DualChannel::new(h.state.tx.clone(), tx);
        let mut session = WsSession::new();

        handle_search(
            &h.state,
            &ch,
            &mut session,
            "search-1".into(),
            "needle".into(),
            10,
            Some(44),
        )
        .await;

        match rx.recv().await {
            Some(ServerMessage::ProtocolError {
                error, scope_nonce, ..
            }) => {
                assert_eq!(error.code, ServerErrorCode::RequestFailed);
                assert_eq!(error.detail.as_deref(), Some("Search feature not enabled"));
                assert_eq!(scope_nonce, Some(44));
            }
            other => panic!("expected ProtocolError, got {:?}", other),
        }
        Ok(())
    }
}

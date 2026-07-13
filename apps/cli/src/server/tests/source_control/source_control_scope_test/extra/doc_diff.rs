use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_doc_diff_uses_shadow_projection() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    let doc_id = DocId::new();
    state.repo.append_remote_ops(
        &peer_id,
        &test_id,
        &[
            shadow_create_file(&peer_id, doc_id, "note.md", 1),
            shadow_insert(&peer_id, doc_id, 0, "hello", 2),
            shadow_insert(&peer_id, doc_id, 5, " remote", 3),
        ],
    )?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(23));

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "doc-req-1".into(),
        ScPathTarget {
            path: "note.md".into(),
            doc_id: Some(doc_id),
        domain: None,
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::DocDiff {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            doc_id: actual_doc_id,
            path,
            projection,
        }) => {
            assert_eq!(request_id.as_deref(), Some("doc-req-1"));
            assert_eq!(repo_id, Some(test_id));
            assert_eq!(branch, Some(peer_id));
            assert_eq!(scope_nonce, Some(23));
            assert_eq!(actual_doc_id, Some(doc_id));
            assert_eq!(path, "note.md");
            assert_eq!(projection.base_content, "");
            assert_eq!(projection.target_content, "hello remote");
        }
        other => panic!("expected DocDiff, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_doc_diff_missing_target_returns_scoped_error() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(29));

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "doc-req-missing".into(),
        ScPathTarget {
            path: "missing.md".into(),
            doc_id: None,
        domain: None,
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScDocNotFound);
            assert_eq!(
                error.detail.as_deref(),
                Some("Remote document not found: missing.md")
            );
            assert_eq!(scope_nonce, Some(29));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_doc_diff_path_mismatch_returns_scoped_error() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    let doc_id = DocId::new();
    state.repo.append_remote_ops(
        &peer_id,
        &test_id,
        &[
            shadow_create_file(&peer_id, doc_id, "note.md", 1),
            shadow_insert(&peer_id, doc_id, 0, "hello", 2),
        ],
    )?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(31));

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "doc-req-mismatch".into(),
        ScPathTarget {
            path: "other.md".into(),
            doc_id: Some(doc_id),
        domain: None,
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert!(error.detail.as_deref().is_some_and(|detail| {
                detail.contains("Remote document target path mismatch")
                    && detail.contains("requested other.md")
                    && detail.contains("is at note.md")
            }));
            assert_eq!(scope_nonce, Some(31));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}

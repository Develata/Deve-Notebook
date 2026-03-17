use super::WsSession;
use super::session_scope::active_db_matches_scope;
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::PeerId;
use std::sync::Arc;

fn handle(repo_name: &str, repo_id: Option<uuid::Uuid>, branch: Option<PeerId>) -> DatabaseHandle {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = Arc::new(redb::Database::create(dir.path().join("repo.redb")).expect("db"));
    DatabaseHandle {
        db,
        readonly: true,
        branch,
        repo_id,
        repo_name: repo_name.into(),
    }
}

#[test]
fn active_db_for_rejects_stale_scope() {
    let mut session = WsSession::new();
    session.set_active_db(handle("notes", None, Some(PeerId::new("peer-a"))));

    assert!(
        session
            .active_db_for(Some(&PeerId::new("peer-a")), "notes", None)
            .is_some()
    );
    assert!(
        session
            .active_db_for(Some(&PeerId::new("peer-b")), "notes", None)
            .is_none()
    );
    assert!(session.active_db_for(None, "notes", None).is_none());
    assert!(
        session
            .active_db_for(Some(&PeerId::new("peer-a")), "other", None)
            .is_none()
    );
}

#[test]
fn active_db_prefers_repo_id_over_name() {
    let repo_id = uuid::Uuid::new_v4();
    let handle = handle("wiki", Some(repo_id), Some(PeerId::new("peer-a")));
    assert!(active_db_matches_scope(&handle, "wiki-1", Some(repo_id)));
    assert!(!active_db_matches_scope(
        &handle,
        "wiki",
        Some(uuid::Uuid::new_v4())
    ));
}

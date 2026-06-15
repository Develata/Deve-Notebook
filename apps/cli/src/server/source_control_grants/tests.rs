use super::{AuthSessionId, SourceControlGrantBranch, SourceControlWriteGrants};
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;
use std::time::Duration;

#[test]
fn grant_authorizes_matching_session_repo_and_scope() {
    let grants = SourceControlWriteGrants::new();
    let auth = AuthSessionId::for_test("session");
    let repo_id = uuid::Uuid::new_v4();
    let writer = PeerId::new("writer");
    grants
        .grant(
            auth.clone(),
            repo_id,
            SourceControlGrantBranch::Local,
            writer.clone(),
            7,
        )
        .unwrap();

    assert_eq!(
        grants.authorize_browser_local(&auth, repo_id, 7).unwrap(),
        writer
    );
}

#[test]
fn grant_rejects_missing_or_stale_scope() {
    let grants = SourceControlWriteGrants::new();
    let auth = AuthSessionId::for_test("session");
    let repo_id = uuid::Uuid::new_v4();
    grants
        .grant(
            auth.clone(),
            repo_id,
            SourceControlGrantBranch::Local,
            PeerId::new("writer"),
            7,
        )
        .unwrap();

    let err = grants
        .authorize_browser_local(&auth, repo_id, 8)
        .unwrap_err();
    assert_eq!(err.code, ServerErrorCode::ScStaleScope);

    let other = AuthSessionId::for_test("other");
    let err = grants
        .authorize_browser_local(&other, repo_id, 7)
        .unwrap_err();
    assert_eq!(err.code, ServerErrorCode::ScStaleScope);
}

#[test]
fn grant_rejects_remote_branch_for_browser_local_authority() {
    let grants = SourceControlWriteGrants::new();
    let auth = AuthSessionId::for_test("session");
    let repo_id = uuid::Uuid::new_v4();
    grants
        .grant(
            auth.clone(),
            repo_id,
            SourceControlGrantBranch::Remote(PeerId::new("remote-peer")),
            PeerId::new("writer"),
            7,
        )
        .unwrap();

    let err = grants
        .authorize_browser_local(&auth, repo_id, 7)
        .unwrap_err();

    assert_eq!(err.code, ServerErrorCode::ScStaleScope);
}

#[test]
fn grant_replaces_previous_session_writer() {
    let grants = SourceControlWriteGrants::new();
    let auth = AuthSessionId::for_test("session");
    let first_repo = uuid::Uuid::new_v4();
    let second_repo = uuid::Uuid::new_v4();
    grants
        .grant(
            auth.clone(),
            first_repo,
            SourceControlGrantBranch::Local,
            PeerId::new("first"),
            1,
        )
        .unwrap();
    grants
        .grant(
            auth.clone(),
            second_repo,
            SourceControlGrantBranch::Local,
            PeerId::new("second"),
            2,
        )
        .unwrap();

    assert!(
        grants
            .authorize_browser_local(&auth, first_repo, 1)
            .is_err()
    );
    assert_eq!(
        grants
            .authorize_browser_local(&auth, second_repo, 2)
            .unwrap(),
        PeerId::new("second")
    );
}

#[test]
fn http_source_control_write_grant_revoked_on_ws_disconnect() {
    let grants = SourceControlWriteGrants::with_ttl(Duration::from_millis(0));
    let auth = AuthSessionId::for_test("session");
    let repo_id = uuid::Uuid::new_v4();
    grants
        .grant(
            auth.clone(),
            repo_id,
            SourceControlGrantBranch::Local,
            PeerId::new("writer"),
            7,
        )
        .unwrap();
    assert!(grants.authorize_browser_local(&auth, repo_id, 7).is_err());

    let grants = SourceControlWriteGrants::new();
    grants
        .grant(
            auth.clone(),
            repo_id,
            SourceControlGrantBranch::Local,
            PeerId::new("writer"),
            7,
        )
        .unwrap();
    grants.revoke_session(&auth);
    assert!(grants.authorize_browser_local(&auth, repo_id, 7).is_err());
}

#[test]
fn source_control_write_grant_creation_fails_closed() {
    let grants = SourceControlWriteGrants::new();
    let auth = AuthSessionId::for_test("session");
    let repo_id = uuid::Uuid::new_v4();

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let poison_result = std::panic::catch_unwind(|| {
        let _guard = grants.grants.lock().expect("grant lock");
        panic!("poison grant lock");
    });
    std::panic::set_hook(previous_hook);
    assert!(poison_result.is_err());

    let err = grants
        .grant(
            auth,
            repo_id,
            SourceControlGrantBranch::Local,
            PeerId::new("writer"),
            7,
        )
        .unwrap_err();

    assert_eq!(err.code, ServerErrorCode::ScStaleScope);
}

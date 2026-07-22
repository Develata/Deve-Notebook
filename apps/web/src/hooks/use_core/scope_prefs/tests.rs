use super::*;

#[test]
fn scope_pref_persists_only_exact_local_repo_identity() {
    let repo_id = uuid::Uuid::new_v4();
    let ScopePrefUpdate::Persist(json) =
        next_scope_pref_json(Some(repo_id.to_string()), None, false)
    else {
        panic!("exact local repo identity should persist");
    };
    assert!(json.contains(&format!("\"repo_id\":\"{repo_id}\"")));
    assert!(json.contains("\"branch\":\"local\""));
    assert!(!json.contains("repo_name"));
    assert!(!json.contains("scope_nonce"));
    assert!(matches!(
        next_scope_pref_json(None, None, false),
        ScopePrefUpdate::Clear
    ));
    assert!(matches!(
        next_scope_pref_json(Some("not-a-repo-id".into()), None, false),
        ScopePrefUpdate::Clear
    ));
    assert!(matches!(
        next_scope_pref_json(
            Some(repo_id.to_string()),
            Some(deve_core::models::PeerId::new("peer-a")),
            false,
        ),
        ScopePrefUpdate::Clear
    ));
}

#[test]
fn parse_scope_pref_recovers_exact_local_repo_only() {
    let repo_id = uuid::Uuid::new_v4();
    let parsed = parse_scope_pref(&serialize_scope_pref(repo_id).unwrap())
        .expect("stored scope should parse");
    assert_eq!(parsed.repo_id, repo_id);
    assert_eq!(parsed.branch, StoredScopeBranchKind::Local);
}

#[test]
fn parse_scope_pref_drops_corrupt_entries() {
    assert!(parse_scope_pref("{\"repo_name\":\"legacy-alias\"}").is_none());
    assert!(parse_scope_pref("{\"repo_id\":\"not-a-uuid\",\"branch\":\"local\"}").is_none());
}

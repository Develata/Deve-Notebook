use super::*;

#[test]
fn scope_pref_requires_complete_uuid_bound_repo() {
    assert!(matches!(
        next_scope_pref_json(
            Some("default".into()),
            Some(uuid::Uuid::new_v4().to_string()),
            None,
            false
        ),
        ScopePrefUpdate::Persist(_)
    ));
    assert!(matches!(
        next_scope_pref_json(
            Some("default".into()),
            Some("not-a-uuid".into()),
            None,
            false
        ),
        ScopePrefUpdate::Skip
    ));
    assert!(matches!(
        next_scope_pref_json(None, None, None, false),
        ScopePrefUpdate::Clear
    ));
}

#[test]
fn parse_scope_pref_recovers_repo_and_branch() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    let parsed = parse_scope_pref(
        &serde_json::to_string(&StoredScopePref {
            repo_name: "default".into(),
            repo_id: repo_id.clone(),
            active_branch: Some("peer-a".into()),
        })
        .unwrap(),
    )
    .expect("stored scope should parse");
    assert_eq!(parsed.repo_name, "default");
    assert_eq!(parsed.repo_id, repo_id);
    assert_eq!(parsed.active_branch.as_deref(), Some("peer-a"));
}

#[test]
fn parse_scope_pref_drops_corrupt_entries() {
    assert!(parse_scope_pref("{\"repo_name\":\"default\",\"repo_id\":\"oops\"}").is_none());
}

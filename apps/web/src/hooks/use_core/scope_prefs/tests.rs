use super::*;

#[test]
fn scope_pref_persists_only_repo_name_alias() {
    assert!(matches!(
        next_scope_pref_json(Some("default".into()), false),
        ScopePrefUpdate::Persist(_)
    ));
    assert!(matches!(
        next_scope_pref_json(None, false),
        ScopePrefUpdate::Clear
    ));
    assert!(matches!(
        next_scope_pref_json(Some("".into()), false),
        ScopePrefUpdate::Clear
    ));
}

#[test]
fn parse_scope_pref_recovers_repo_alias_only() {
    let parsed = parse_scope_pref(
        &serde_json::to_string(&StoredScopePref {
            repo_name: "default".into(),
        })
        .unwrap(),
    )
    .expect("stored scope should parse");
    assert_eq!(parsed.repo_name, "default");
}

#[test]
fn parse_scope_pref_drops_corrupt_entries() {
    assert!(parse_scope_pref("{\"repo_name\":\"\"}").is_none());
}

use super::RepoManager;

#[test]
fn list_loaded_shadows_fails_closed_when_registry_lock_is_poisoned() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;

    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = repo.write_shadow_dbs().expect("write lock");
        panic!("poison shadow registry");
    }));

    let err = repo
        .list_loaded_shadows()
        .expect_err("must fail closed after lock poison");
    assert!(err.to_string().contains("Shadow DB registry lock poisoned"));
    Ok(())
}

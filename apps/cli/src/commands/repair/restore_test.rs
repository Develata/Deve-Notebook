use super::{
    find_loading_corruption, normalize_restore_path, resolve_backup_path, resolve_repair_docid,
    workspace_starts_with_loading,
};
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::DocId;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn resolve_repair_docid_returns_tracked_docid() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let repo = Arc::new(RepoManager::init(
        dir.path(),
        10,
        Some("default"),
        Some("urn:default"),
    )?);
    let doc_id = DocId::new();
    repo.apply_file_structure_in_local_repo("default", "notes/live.md", Some(doc_id), "test")?;
    assert_eq!(
        resolve_repair_docid(&repo, "default", "notes/live.md")?,
        Some(doc_id)
    );
    Ok(())
}

#[test]
fn resolve_repair_docid_fails_closed_for_legacy_only_path_mapping() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let repo = Arc::new(RepoManager::init(
        dir.path(),
        10,
        Some("default"),
        Some("urn:default"),
    )?);
    let doc_id = DocId::new();
    repo.run_on_local_repo("default", |db| {
        let write = db.begin_write()?;
        {
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/legacy.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/legacy.md")?;
        }
        write.commit()?;
        Ok(())
    })?;

    let err = resolve_repair_docid(&repo, "default", "notes/legacy.md")
        .expect_err("legacy-only repair target must fail closed");
    assert!(
        err.to_string()
            .contains("Tracked document projection missing for legacy-mapped path")
    );
    Ok(())
}

#[test]
fn resolve_backup_path_requires_repo_scoped_backup_layout() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let direct = dir.path().join("notes/live.md");
    std::fs::create_dir_all(direct.parent().expect("parent"))?;
    std::fs::write(&direct, "backup")?;

    assert_eq!(
        resolve_backup_path(dir.path(), "default", "notes/live.md"),
        None
    );
    Ok(())
}

#[test]
fn normalize_restore_path_strips_repo_prefix() {
    assert_eq!(
        normalize_restore_path("default", "default/notes/live.md"),
        "notes/live.md"
    );
    assert_eq!(
        normalize_restore_path("default", "notes/live.md"),
        "notes/live.md"
    );
}

#[test]
fn find_loading_corruption_uses_tracked_docs_only() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(dir.path());
    let repo = Arc::new(repo);
    let doc_id = DocId::new();
    repo.apply_file_structure_in_local_repo("default", "notes/live.md", Some(doc_id), "test")?;
    let live = repo.local_repo_workspace_path("default", "notes/live.md")?;
    let stale = repo.local_repo_workspace_path("default", "notes/stale.md")?;
    std::fs::create_dir_all(live.parent().expect("live parent"))?;
    std::fs::create_dir_all(stale.parent().expect("stale parent"))?;
    std::fs::write(&live, "# Loading...\n")?;
    std::fs::write(&stale, "# Loading...\n")?;

    let mut targets = find_loading_corruption(&repo, "default")?;
    targets.sort();
    assert_eq!(targets, vec!["notes/live.md".to_string()]);
    assert!(workspace_starts_with_loading(
        &repo,
        "default",
        "notes/live.md"
    )?);
    Ok(())
}

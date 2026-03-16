use super::remote::{local_counterpart_content, resolve_remote_content, resolve_tracked_doc_id};
use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID};
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::models::PeerId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use deve_core::{config::SyncMode, protocol::ServerMessage};
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

fn new_repo() -> anyhow::Result<(TempDir, RepoManager)> {
    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
    repo.set_vault_root(dir.path().join("vault"));
    Ok((dir, repo))
}

fn build_state(dir: &TempDir, repo: RepoManager) -> anyhow::Result<Arc<AppState>> {
    let vault = dir.path().join("vault");
    let repo = Arc::new(repo);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok(Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
        tx: broadcast::channel::<ServerMessage>(16).0,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            PeerId::new("test-peer"),
            repo,
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_service: None,
        identity_key,
    }))
}

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn seed_pending_entry(repo: &RepoManager, entry: PendingFsEntry) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| pending_fs::upsert(db, &entry))
        .expect("seed pending entry");
}

#[test]
fn remote_diff_prefers_doc_id_for_local_counterpart() -> anyhow::Result<()> {
    let (dir, repo) = new_repo()?;
    let selector = RepoSelector::default();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 1,
            has_conflict: false,
        },
    );
    repo.stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/a.md"))?;
    repo.commit_staged_in_repo(&selector, "initial")?;
    let doc_id = repo.get_docid("notes/a.md")?.expect("existing doc id");

    std::fs::remove_file(dir.path().join("vault").join("default").join("notes/a.md"))?;
    write_workspace_file(&dir, "notes/b.md", "hello renamed");
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Deleted,
            content_hash: String::new(),
            detected_at: 2,
            has_conflict: false,
        },
    );
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/b.md".into(),
            renamed_from: Some("notes/a.md".into()),
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello renamed"),
            detected_at: 2,
            has_conflict: false,
        },
    );
    repo.stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/b.md"))?;
    repo.commit_staged_in_repo(&selector, "rename")?;

    let content = local_counterpart_content(&repo, doc_id, Some(repo.local_repo_name()))?;
    assert_eq!(content.as_deref(), Some("hello renamed"));
    Ok(())
}

#[test]
fn remote_diff_prefers_node_projection_before_legacy_path_mapping() -> anyhow::Result<()> {
    let (dir, repo) = new_repo()?;
    let selector = RepoSelector::default();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 1,
            has_conflict: false,
        },
    );
    repo.stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/a.md"))?;
    repo.commit_staged_in_repo(&selector, "initial")?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        {
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            d2p.retain(|_, _| false)?;
            p2d.retain(|_, _| false)?;
        }
        write.commit()?;
        Ok(())
    })?;

    let doc_id = repo.run_on_local_repo(repo.local_repo_name(), |db| {
        resolve_tracked_doc_id(db, &ScPathTarget::from_path("notes/a.md"))
    })?;
    assert!(doc_id.is_some());
    Ok(())
}

#[test]
fn remote_diff_fails_closed_on_legacy_only_path_mapping() -> anyhow::Result<()> {
    let (dir, repo) = new_repo()?;
    let selector = RepoSelector::default();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 1,
            has_conflict: false,
        },
    );
    repo.stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/a.md"))?;
    repo.commit_staged_in_repo(&selector, "initial")?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        {
            let mut p2n = write.open_table(PATH_TO_NODEID)?;
            let mut n2m = write.open_table(NODEID_TO_META)?;
            p2n.retain(|_, _| false)?;
            n2m.retain(|_, _| false)?;
        }
        write.commit()?;
        Ok(())
    })?;

    let err = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            resolve_tracked_doc_id(db, &ScPathTarget::from_path("notes/a.md"))
        })
        .expect_err("legacy-only path mapping must fail closed");
    assert!(
        err.to_string()
            .contains("Tracked document projection missing for legacy-mapped path")
    );
    Ok(())
}

#[test]
fn remote_diff_rejects_deleted_doc_even_with_doc_id_hint() -> anyhow::Result<()> {
    let (dir, repo) = new_repo()?;
    let selector = RepoSelector::default();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 1,
            has_conflict: false,
        },
    );
    repo.stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/a.md"))?;
    repo.commit_staged_in_repo(&selector, "initial")?;
    let doc_id = repo.get_docid("notes/a.md")?.expect("existing doc id");
    repo.apply_file_delete_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        Some(doc_id),
        "test",
    )?;

    let resolved = repo.run_on_local_repo(repo.local_repo_name(), |db| {
        resolve_tracked_doc_id(
            db,
            &ScPathTarget {
                path: "notes/a.md".into(),
                doc_id: Some(doc_id),
            },
        )
    })?;
    assert!(resolved.is_none());
    assert!(local_counterpart_content(&repo, doc_id, Some(repo.local_repo_name()))?.is_none());
    Ok(())
}

#[test]
fn remote_diff_surfaces_shadow_lookup_errors_instead_of_not_found() -> anyhow::Result<()> {
    let (dir, repo) = new_repo()?;
    let peer = PeerId::new("peer-missing");
    let repo_id = uuid::Uuid::new_v4();
    std::fs::create_dir_all(
        dir.path()
            .join("remotes")
            .join(peer.to_filename())
            .join(format!("{}.redb", repo_id)),
    )?;
    let state = build_state(&dir, repo)?;
    let err = resolve_remote_content(
        &state,
        Some(&peer),
        repo_id,
        &ScPathTarget::from_path("notes/a.md"),
    )
    .expect_err("missing shadow repo should stay an error");
    let detail = err.to_string();
    assert!(
        detail.contains("shadow")
            || detail.contains("redb")
            || detail.contains("directory")
            || detail.contains("Is a directory"),
        "unexpected error detail: {detail}"
    );
    Ok(())
}

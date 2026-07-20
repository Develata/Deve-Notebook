//! plan_ref:
//!   - 04_repository#host-repo-alias-contract
//!   - 14_commands#repo-alias-command-contract

use super::*;
use crate::ledger::RepoManager;
use crate::models::RepoId;
use serde_json::json;

mod security;

fn init_alias_repo(ledger: &std::path::Path) -> anyhow::Result<RepoManager> {
    let repo_id = RepoId::new_v4();
    let execution_name = repo_id.to_string();
    let repo = RepoManager::init_with_options(
        ledger,
        8,
        Some(&execution_name),
        crate::ledger::init::RepoInitOptions {
            repo_id: Some(repo_id),
            repo_url: None,
        },
    )?;
    let projection = ledger
        .parent()
        .expect("test ledger has parent")
        .join("alias-test-projections");
    let locator = repo.prepare_projection_locator_for_repo_creation(repo_id, &projection)?;
    let workspace = locator.projection_base_abs.join(&locator.workspace_segment);
    std::fs::create_dir_all(&workspace)?;
    crate::utils::notegit::ensure_repo_identity_marker(&workspace, repo_id, &execution_name)?;
    repo.seed_catalog_membership_from_records()?;
    let authority = repo.claim_repo_catalog_cut_authority()?;
    let prepared = repo.prepare_repo_creation_membership(repo_id, uuid::Uuid::new_v4())?;
    let revalidated = repo.revalidate_repo_creation_membership(&prepared)?;
    let permit = authority.permit(repo_id)?;
    repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;
    Ok(repo)
}

fn repo_ids(repo: &RepoManager) -> anyhow::Result<Vec<RepoId>> {
    Ok(repo
        .list_cataloged_local_repo_summaries()?
        .into_iter()
        .map(|summary| summary.repo_id)
        .collect())
}

fn import_document(entries: Vec<serde_json::Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "format": "deve.host-repo-aliases",
        "version": 1,
        "aliases": entries,
    }))
    .expect("serialize import document")
}

#[test]
fn missing_alias_falls_back_to_full_repo_id_and_set_is_cas() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let repo = init_alias_repo(&dir.path().join("ledger"))?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let runtime = repo.host_repo_alias_runtime();

    let fallback = runtime.binding(repo_id)?;
    assert_eq!(fallback.alias, repo_id.to_string());
    assert_eq!(fallback.alias_revision, 0);

    let first = runtime.set_alias(repo_id, "  math / 📐  ", 0)?;
    assert!(first.changed);
    assert_eq!(first.binding.alias, "math / 📐");
    assert_eq!(first.binding.alias_revision, 1);

    let same = runtime.set_alias(repo_id, "math / 📐", 1)?;
    assert!(!same.changed);
    assert_eq!(same.binding.alias_revision, 1);

    let conflict = runtime
        .set_alias(repo_id, "other", 0)
        .expect_err("stale CAS must fail");
    assert!(matches!(
        conflict,
        HostRepoAliasError::RevisionConflict {
            expected: 0,
            current: 1,
            ..
        }
    ));
    assert_eq!(runtime.binding(repo_id)?.alias, "math / 📐");
    Ok(())
}

#[test]
fn alias_validation_is_display_only_but_rejects_empty_control_and_oversize() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let repo = init_alias_repo(&dir.path().join("ledger"))?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let runtime = repo.host_repo_alias_runtime();

    assert!(matches!(
        runtime.set_alias(repo_id, "  ", 0),
        Err(HostRepoAliasError::InvalidAlias(
            HostRepoAliasValidationError::Empty
        ))
    ));
    assert!(matches!(
        runtime.set_alias(repo_id, "line\nbreak", 0),
        Err(HostRepoAliasError::InvalidAlias(
            HostRepoAliasValidationError::ContainsControl
        ))
    ));
    assert!(matches!(
        runtime.set_alias(repo_id, &"x".repeat(257), 0),
        Err(HostRepoAliasError::InvalidAlias(
            HostRepoAliasValidationError::TooLong
        ))
    ));
    assert!(runtime.set_alias(repo_id, r"math/notes\local", 0)?.changed);
    Ok(())
}

#[test]
fn export_is_deterministic_and_contains_only_explicit_active_aliases() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = init_alias_repo(&ledger)?;
    init_alias_repo(&ledger)?;
    let mut ids = repo_ids(&repo)?;
    ids.sort();
    let runtime = repo.host_repo_alias_runtime();
    runtime.set_alias(ids[1], "zeta", 0)?;
    runtime.set_alias(ids[0], "alpha", 0)?;

    let exported = runtime.export_json()?;
    let value: serde_json::Value = serde_json::from_str(&exported)?;
    assert_eq!(value["format"], "deve.host-repo-aliases");
    assert_eq!(value["version"], 1);
    assert_eq!(value["aliases"].as_array().expect("aliases").len(), 2);
    assert_eq!(value["aliases"][0]["repo_id"], ids[0].to_string());
    assert_eq!(value["aliases"][1]["repo_id"], ids[1].to_string());
    assert!(!exported.contains("revision"));
    assert!(!exported.contains("path"));
    assert_eq!(exported, runtime.export_json()?);
    Ok(())
}

#[test]
fn import_reports_every_bad_entry_skips_all_duplicates_and_commits_good_batch() -> anyhow::Result<()>
{
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = init_alias_repo(&ledger)?;
    init_alias_repo(&ledger)?;
    init_alias_repo(&ledger)?;
    init_alias_repo(&ledger)?;
    let ids = repo_ids(&repo)?;
    let good = ids[0];
    let duplicate = ids[1];
    let unknown = RepoId::new_v4();
    let input = import_document(vec![
        json!({"repo_id": good, "alias": "  good  "}),
        json!({"repo_id": duplicate, "alias": "first"}),
        json!({"repo_id": duplicate, "alias": "last"}),
        json!({"repo_id": unknown, "alias": "foreign"}),
        json!({"repo_id": ids[2], "alias": "bad\nname"}),
        json!({"repo_id": "not-a-uuid", "alias": "bad id"}),
        json!({"repo_id": ids[3]}),
        json!(false),
    ]);
    let runtime = repo.host_repo_alias_runtime();

    let preview = runtime.preview_import_json(&input)?;
    assert_eq!(preview.accepted, 1);
    assert_eq!(preview.changed, 1);
    assert_eq!(preview.unchanged, 0);
    assert_eq!(preview.skipped, 7);
    assert_eq!(preview.warnings.len(), 7);
    assert_eq!(preview.warnings[0].index, 1);
    assert_eq!(
        preview.warnings[0].reason,
        HostRepoAliasImportWarningReason::DuplicateRepoId
    );
    assert_eq!(preview.warnings[1].index, 2);
    assert_eq!(
        preview
            .warnings
            .iter()
            .find(|warning| warning.index == 4)
            .expect("control-character warning")
            .reason,
        HostRepoAliasImportWarningReason::AliasContainsControl
    );
    assert_eq!(runtime.binding(good)?.alias_revision, 0);

    let applied = runtime.apply_import_json(&input)?;
    assert_eq!(applied, preview);
    assert_eq!(runtime.binding(good)?.alias, "good");
    assert_eq!(runtime.binding(good)?.alias_revision, 1);
    assert_eq!(runtime.binding(duplicate)?.alias_revision, 0);
    assert_eq!(runtime.binding(ids[2])?.alias_revision, 0);
    Ok(())
}

#[test]
fn apply_revalidates_against_latest_store_after_preview() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let repo = init_alias_repo(&dir.path().join("ledger"))?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let input = import_document(vec![json!({"repo_id": repo_id, "alias": "imported"})]);
    let runtime = repo.host_repo_alias_runtime();

    let preview = runtime.preview_import_json(&input)?;
    assert_eq!(preview.changed, 1);
    runtime.set_alias(repo_id, "concurrent", 0)?;
    let applied = runtime.apply_import_json(&input)?;

    assert_eq!(applied.changed, 1);
    let binding = runtime.binding(repo_id)?;
    assert_eq!(binding.alias, "imported");
    assert_eq!(binding.alias_revision, 2);
    Ok(())
}

#[test]
fn apply_revalidates_membership_after_preview() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = init_alias_repo(&ledger)?;
    init_alias_repo(&ledger)?;
    let primary = repo.get_repo_info()?.expect("primary").uuid;
    let target = repo
        .list_cataloged_local_repo_summaries()?
        .into_iter()
        .find(|summary| summary.repo_id != primary)
        .expect("secondary repo")
        .repo_id;
    let input = import_document(vec![json!({"repo_id": target, "alias": "temporary"})]);
    let runtime = repo.host_repo_alias_runtime();
    assert_eq!(runtime.preview_import_json(&input)?.accepted, 1);

    repo.seed_catalog_membership_from_records()?;
    let authority = repo.claim_repo_catalog_cut_authority()?;
    let prepared = repo.prepare_repo_removal_membership(target, uuid::Uuid::new_v4())?;
    let revalidated = repo.revalidate_repo_removal_membership(&prepared)?;
    let permit = authority.permit(target)?;
    repo.commit_repo_removal_membership(&prepared, &revalidated, &permit)?;
    let applied = runtime.apply_import_json(&input)?;

    assert_eq!(applied.accepted, 0);
    assert_eq!(applied.skipped, 1);
    assert_eq!(
        applied.warnings[0].reason,
        HostRepoAliasImportWarningReason::UnknownLocalRepo
    );
    Ok(())
}

#[test]
fn concurrent_cas_across_runtime_instances_has_one_winner() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = init_alias_repo(&ledger)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let mut workers = Vec::new();
    for alias in ["alpha", "beta"] {
        let ledger = ledger.clone();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            let runtime = HostRepoAliasRuntime::open_existing(ledger).expect("open alias runtime");
            barrier.wait();
            runtime.set_alias(repo_id, alias, 0)
        }));
    }
    barrier.wait();
    let results = workers
        .into_iter()
        .map(|worker| worker.join().expect("worker join"))
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(HostRepoAliasError::RevisionConflict { .. })))
            .count(),
        1
    );
    Ok(())
}

#[test]
fn standalone_membership_checks_do_not_retain_repo_database_handles() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = init_alias_repo(&ledger)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    drop(repo);
    crate::ledger::database_cache::evict_database_paths_under(&ledger)?;

    let runtime = HostRepoAliasRuntime::open_existing(&ledger)?;
    runtime.set_alias(repo_id, "standalone", 0)?;
    assert_eq!(runtime.binding(repo_id)?.alias, "standalone");

    let cache = crate::ledger::database_cache::OPENED_DBS
        .read()
        .expect("database cache");
    assert!(cache.keys().all(|path| !path.starts_with(&ledger)));
    Ok(())
}

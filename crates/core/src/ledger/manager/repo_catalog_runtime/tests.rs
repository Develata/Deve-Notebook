//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 03_storage/index#repo-runtime-layout

use super::*;
use crate::ledger::RepoManager;
use crate::models::RepoId;
use tempfile::TempDir;
use uuid::Uuid;

const REMOVAL_MANIFEST_DIGEST: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

struct Fixture {
    _temp: TempDir,
    repo: RepoManager,
    repo_id: RepoId,
    marker: std::path::PathBuf,
}

fn fixture() -> anyhow::Result<Fixture> {
    let temp = tempfile::tempdir()?;
    let ledger = temp.path().join("ledger");
    let projection_base = temp.path().join("workspaces");
    std::fs::create_dir_all(&projection_base)?;
    // Prepared (uncommitted) catalog repo: UUID-canonical machine name, prepared
    // locator, created workspace + identity marker, and process membership seeded
    // from durable records (empty). These tests drive the create/remove cut
    // themselves, so no membership record is committed here.
    let prepared =
        crate::test_support::prepare_cataloged_repo(&ledger, &projection_base, 16, None)?;
    let marker = crate::utils::notegit::repo_identity_path(&prepared.workspace);
    Ok(Fixture {
        _temp: temp,
        repo: prepared.repo,
        repo_id: prepared.repo_id,
        marker,
    })
}

fn commit_creation(
    repo: &RepoManager,
    prepared: &PreparedRepoCreation,
) -> Result<RepoCatalogCreationCommit, RepoCatalogError> {
    let revalidated = repo.revalidate_repo_creation_membership(prepared)?;
    let permit = repo
        .catalog_membership_runtime()
        .cut_permit_for_test(prepared.repo_id());
    let commit = repo.commit_repo_creation_membership(prepared, &revalidated, &permit)?;
    repo.activate_initial_prepared_local_repo_authority(prepared, &commit)
        .map_err(|error| RepoCatalogError::PreparedIdentityUnavailable {
            repo_id: prepared.repo_id(),
            detail: error.to_string(),
        })?;
    Ok(commit)
}

fn commit_removal(
    repo: &RepoManager,
    prepared: &PreparedRepoRemoval,
) -> Result<RepoCatalogRemovalCommit, RepoCatalogError> {
    let revalidated = repo.revalidate_repo_removal_membership(prepared)?;
    let permit = repo
        .catalog_membership_runtime()
        .cut_permit_for_test(prepared.repo_id());
    repo.commit_repo_removal_membership(prepared, &revalidated, &permit)
}

#[test]
fn create_cut_is_durable_idempotent_and_rotates_process_membership() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let request_id = Uuid::new_v4();
    let prepared = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, request_id)?;
    assert_eq!(prepared.repo_id(), fixture.repo_id);

    let committed = commit_creation(&fixture.repo, &prepared)?;
    assert_eq!(
        committed.record().state(),
        RepoCatalogMembershipState::Normal
    );
    assert_eq!(committed.record().membership_revision(), 1);
    assert_eq!(committed.record().lifecycle_request_id(), request_id);
    fixture
        .repo
        .catalog_membership_runtime()
        .revalidate(committed.membership())?;
    assert_eq!(fixture.repo.normal_repo_catalog_ids()?, [fixture.repo_id]);

    let repeated = commit_creation(&fixture.repo, &prepared)?;
    assert_eq!(repeated.record(), committed.record());
    assert_eq!(repeated.membership(), committed.membership());
    Ok(())
}

#[test]
fn create_cut_rejects_invalid_workspace_identity_without_record() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let prepared = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    let marker = format!(
        "version = 1\nrepo_id = \"{}\"\nrepo_name = \"host-local-only\"\n",
        RepoId::new_v4()
    );
    std::fs::write(&fixture.marker, marker)?;

    assert!(matches!(
        commit_creation(&fixture.repo, &prepared),
        Err(RepoCatalogError::PreparedIdentityUnavailable { repo_id, .. }) if repo_id == fixture.repo_id
    ));
    assert!(
        fixture
            .repo
            .repo_catalog_membership_record(fixture.repo_id)?
            .is_none()
    );
    Ok(())
}

#[test]
fn catalog_cut_rejects_a_permit_for_another_repo() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let prepared = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    let revalidated = fixture
        .repo
        .revalidate_repo_creation_membership(&prepared)?;
    let permit = fixture
        .repo
        .catalog_membership_runtime()
        .cut_permit_for_test(RepoId::new_v4());

    assert!(matches!(
        fixture
            .repo
            .commit_repo_creation_membership(&prepared, &revalidated, &permit),
        Err(RepoCatalogError::Membership(
            CatalogMembershipError::CutPermitMismatch(repo_id)
        )) if repo_id == fixture.repo_id
    ));
    assert!(
        fixture
            .repo
            .repo_catalog_membership_record(fixture.repo_id)?
            .is_none()
    );
    Ok(())
}

#[test]
fn nonsemantic_marker_text_does_not_change_machine_identity() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let prepared = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    let mut marker = std::fs::read_to_string(&fixture.marker)?;
    marker.push_str("\n# host-local display metadata is not machine identity\n");
    std::fs::write(&fixture.marker, marker)?;

    let committed = commit_creation(&fixture.repo, &prepared)?;
    assert_eq!(committed.record().repo_id(), fixture.repo_id);
    Ok(())
}

#[test]
fn remove_cut_is_conditional_idempotent_and_invalidates_old_token() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let created = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    commit_creation(&fixture.repo, &created)?;
    let remove_request = Uuid::new_v4();
    let prepared = fixture.repo.prepare_repo_removal_membership(
        fixture.repo_id,
        remove_request,
        REMOVAL_MANIFEST_DIGEST,
    )?;
    let old_token = prepared.membership().clone();

    let committed = commit_removal(&fixture.repo, &prepared)?;
    assert_eq!(
        committed.record().state(),
        RepoCatalogMembershipState::Removed
    );
    assert_eq!(committed.record().membership_revision(), 2);
    assert_eq!(committed.record().lifecycle_request_id(), remove_request);
    assert!(
        fixture
            .repo
            .catalog_membership_runtime()
            .revalidate(&old_token)
            .is_err()
    );
    assert!(fixture.repo.normal_repo_catalog_ids()?.is_empty());

    let repeated = commit_removal(&fixture.repo, &prepared)?;
    assert_eq!(repeated.record(), committed.record());
    Ok(())
}

#[test]
fn removal_uses_current_relocation_snapshot_not_historical_create_digest() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let create = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    commit_creation(&fixture.repo, &create)?;
    let relocated_base = fixture._temp.path().join("relocated-workspaces");
    std::fs::create_dir_all(&relocated_base)?;
    let execution = fixture.repo.local_repo_name().to_string();
    fixture
        .repo
        .set_projection_base_for_local_repo(&execution, &relocated_base)?;
    let relocated = fixture.repo.local_repo_workspace_root(&execution)?;
    std::fs::create_dir_all(&relocated)?;
    fixture
        .repo
        .ensure_local_repo_workspace_identity(&execution)?;

    let prepared = fixture.repo.prepare_repo_removal_membership(
        fixture.repo_id,
        Uuid::new_v4(),
        REMOVAL_MANIFEST_DIGEST,
    )?;
    assert_ne!(
        prepared.prepared_identity().to_hex(),
        prepared.normal_record.prepared_identity_digest()
    );
    commit_removal(&fixture.repo, &prepared)?;
    Ok(())
}

#[test]
fn remove_cut_rejects_record_changed_after_prepare() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let created = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    commit_creation(&fixture.repo, &created)?;
    let prepared = fixture.repo.prepare_repo_removal_membership(
        fixture.repo_id,
        Uuid::new_v4(),
        REMOVAL_MANIFEST_DIGEST,
    )?;

    let store = store::RepoCatalogStore::open(fixture.repo.ledger_dir())?;
    let _store_lock = store.lock()?;
    let mut changed = prepared.normal_record.clone();
    changed.membership_revision += 1;
    changed.lifecycle_request_id = Uuid::new_v4();
    store.publish(&changed)?;
    drop(_store_lock);

    assert!(matches!(
        commit_removal(&fixture.repo, &prepared),
        Err(RepoCatalogError::CutOutcomeUnknown { repo_id, .. }) if repo_id == fixture.repo_id
    ));
    fixture
        .repo
        .catalog_membership_runtime()
        .revalidate(prepared.membership())?;
    Ok(())
}

#[test]
fn second_manager_for_same_repo_is_rejected_by_authority_owner() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let prepared = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    let committed = commit_creation(&fixture.repo, &prepared)?;
    let old_token = committed.membership().clone();

    let error = match RepoManager::init_existing_for_repo_id(
        fixture.repo.ledger_dir(),
        fixture.repo.snapshot_depth(),
        fixture.repo_id,
    ) {
        Ok(_) => panic!("same RepoId must have one live authority owner"),
        Err(error) => error,
    };
    assert!(format!("{error:#}").contains("busy"));
    fixture
        .repo
        .catalog_membership_runtime()
        .revalidate(&old_token)?;
    Ok(())
}

#[test]
fn removed_catalog_record_cannot_reanimate_a_residual_database() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let ledger = fixture.repo.ledger_dir().to_path_buf();
    let repo_id = fixture.repo_id;
    let created = fixture
        .repo
        .prepare_repo_creation_membership(repo_id, Uuid::new_v4())?;
    commit_creation(&fixture.repo, &created)?;
    let removed = fixture.repo.prepare_repo_removal_membership(
        repo_id,
        Uuid::new_v4(),
        REMOVAL_MANIFEST_DIGEST,
    )?;
    commit_removal(&fixture.repo, &removed)?;
    drop(fixture.repo);

    let error = match RepoManager::init(&ledger, 8, None, None) {
        Ok(_) => panic!("Removed catalog member must not be selected from a residual database"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("No durable Normal local repository")
    );
    let empty = RepoManager::init_empty_host(&ledger, 8)?;
    assert!(empty.list_cataloged_local_repo_summaries()?.is_empty());
    assert!(
        ledger
            .join("local")
            .join(format!("{repo_id}.redb"))
            .is_file(),
        "R2 composes NoScope without admitting or deleting committed cleanup debt"
    );
    Ok(())
}

#[test]
fn uncataloged_prepared_database_requires_explicit_repair_after_restart() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let ledger = temp.path().join("ledger");
    let prepared = RepoManager::init(&ledger, 8, Some("notes"), Some("urn:notes"))?;
    drop(prepared);

    let error = match RepoManager::init(&ledger, 8, Some("notes"), Some("urn:notes")) {
        Ok(_) => panic!("normal init must not infer membership from an uncataloged database"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("Uncataloged local authority artifacts require explicit ownership repair")
    );
    Ok(())
}

#[test]
fn catalog_file_lock_serializes_independent_store_handles() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let first = store::RepoCatalogStore::open(fixture.repo.ledger_dir())?;
    let second = store::RepoCatalogStore::open(fixture.repo.ledger_dir())?;
    let first_guard = first.lock()?;

    assert!(matches!(
        second.lock(),
        Err(RepoCatalogError::AuthorityBusy)
    ));

    drop(first_guard);
    let _second_guard = second.lock()?;
    Ok(())
}

#[test]
fn standalone_normal_catalog_probe_respects_the_store_lock() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let store = store::RepoCatalogStore::open(fixture.repo.ledger_dir())?;
    let guard = store.lock()?;

    assert!(matches!(
        normal_catalog_ids_for_ledger(fixture.repo.ledger_dir()),
        Err(RepoCatalogError::AuthorityBusy)
    ));

    drop(guard);
    assert!(normal_catalog_ids_for_ledger(fixture.repo.ledger_dir())?.is_empty());
    Ok(())
}

#[path = "tests/failure.rs"]
mod failure;

#[test]
fn catalog_record_tamper_fails_closed() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let prepared = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    commit_creation(&fixture.repo, &prepared)?;
    let path = crate::utils::notegit::host_dir(fixture.repo.ledger_dir())
        .join("repo-catalog")
        .join(format!("{}.json", fixture.repo_id));
    let mut bytes = std::fs::read(&path)?;
    bytes.extend_from_slice(b" \n");
    std::fs::write(&path, bytes)?;

    assert!(matches!(
        fixture.repo.repo_catalog_membership_record(fixture.repo_id),
        Err(RepoCatalogError::InvalidRecord(_))
    ));
    Ok(())
}

#[test]
fn catalog_v1_record_fails_closed() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let prepared = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    commit_creation(&fixture.repo, &prepared)?;
    let path = crate::utils::notegit::host_dir(fixture.repo.ledger_dir())
        .join("repo-catalog")
        .join(format!("{}.json", fixture.repo_id));
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    value["version"] = serde_json::json!(1);
    std::fs::write(&path, serde_json::to_vec(&value)?)?;

    assert!(matches!(
        fixture.repo.repo_catalog_membership_record(fixture.repo_id),
        Err(RepoCatalogError::InvalidRecord(detail))
            if detail.contains("unsupported version 1")
    ));
    Ok(())
}

#[test]
fn conflicting_create_requests_cannot_overwrite_the_first_cut() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let first = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    let second = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    let committed = commit_creation(&fixture.repo, &first)?;

    assert!(matches!(
        commit_creation(&fixture.repo, &second),
        Err(RepoCatalogError::AlreadyExists(repo_id)) if repo_id == fixture.repo_id
    ));
    assert_eq!(
        fixture
            .repo
            .repo_catalog_membership_record(fixture.repo_id)?
            .as_ref(),
        Some(committed.record())
    );
    Ok(())
}

#[test]
fn oversized_or_unexpected_catalog_entries_fail_closed() -> anyhow::Result<()> {
    let fixture = fixture()?;
    assert!(fixture.repo.normal_repo_catalog_ids()?.is_empty());
    let catalog = crate::utils::notegit::host_dir(fixture.repo.ledger_dir()).join("repo-catalog");
    std::fs::write(catalog.join("unexpected.tmp"), b"orphan")?;
    assert!(matches!(
        fixture.repo.normal_repo_catalog_ids(),
        Err(RepoCatalogError::InvalidRecord(_))
    ));
    std::fs::remove_file(catalog.join("unexpected.tmp"))?;

    let owned_temp = catalog.join(format!(
        ".deve-repo-catalog.{}.999.{}.tmp",
        fixture.repo_id,
        Uuid::new_v4()
    ));
    std::fs::write(&owned_temp, b"crash debris")?;
    assert!(fixture.repo.normal_repo_catalog_ids()?.is_empty());
    assert!(!owned_temp.exists());

    let record = catalog.join(format!("{}.json", fixture.repo_id));
    std::fs::write(
        &record,
        vec![b'x'; (model::CATALOG_RECORD_MAX_BYTES + 1) as usize],
    )?;
    assert!(matches!(
        fixture.repo.repo_catalog_membership_record(fixture.repo_id),
        Err(RepoCatalogError::InvalidRecord(_))
    ));
    Ok(())
}

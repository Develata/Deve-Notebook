//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 03_storage/index#repo-runtime-layout

use super::*;
use crate::ledger::RepoManager;
use crate::ledger::init::RepoInitOptions;
use crate::models::RepoId;
use tempfile::TempDir;
use uuid::Uuid;

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
    let repo_id = RepoId::new_v4();
    let repo = RepoManager::init_with_options(
        &ledger,
        16,
        Some("prepared-local-alias"),
        RepoInitOptions {
            repo_id: Some(repo_id),
            repo_url: None,
        },
    )?;
    let execution = repo.local_repo_name().to_string();
    repo.set_projection_base_for_local_repo(&execution, &projection_base)?;
    let workspace = repo.local_repo_workspace_root(&execution)?;
    std::fs::create_dir_all(&workspace)?;
    repo.ensure_local_repo_workspace_identity(&execution)?;
    repo.catalog_membership_runtime().seed([])?;
    Ok(Fixture {
        _temp: temp,
        repo,
        repo_id,
        marker: crate::utils::notegit::repo_identity_path(&workspace),
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
    repo.commit_repo_creation_membership(prepared, &revalidated, &permit)
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
    let prepared = fixture
        .repo
        .prepare_repo_removal_membership(fixture.repo_id, remove_request)?;
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

    let prepared = fixture
        .repo
        .prepare_repo_removal_membership(fixture.repo_id, Uuid::new_v4())?;
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
    let prepared = fixture
        .repo
        .prepare_repo_removal_membership(fixture.repo_id, Uuid::new_v4())?;

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
fn managers_for_one_ledger_share_the_single_process_membership_authority() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let prepared = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    let committed = commit_creation(&fixture.repo, &prepared)?;
    let old_token = committed.membership().clone();

    let reopened = RepoManager::init_existing_for_repo_id(
        fixture.repo.ledger_dir(),
        fixture.repo.snapshot_depth(),
        fixture.repo_id,
    )?;
    reopened.seed_catalog_membership_from_records()?;
    let new_token = reopened
        .catalog_membership_runtime()
        .issue(fixture.repo_id)?;
    assert!(
        reopened
            .catalog_membership_runtime()
            .revalidate(&new_token)
            .is_ok()
    );
    reopened
        .catalog_membership_runtime()
        .revalidate(&old_token)?;
    assert_eq!(new_token, old_token);
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

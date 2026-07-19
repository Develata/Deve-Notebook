//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator

use super::*;

#[test]
fn post_replace_create_failure_stays_fail_closed_until_retry_seals_directory() -> anyhow::Result<()>
{
    let fixture = fixture()?;
    let prepared = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    let store = store::RepoCatalogStore::open(fixture.repo.ledger_dir())?;
    let marker = store.post_replace_failure_marker();
    std::fs::write(&marker, b"inject")?;

    assert!(matches!(
        commit_creation(&fixture.repo, &prepared),
        Err(RepoCatalogError::CutOutcomeUnknown { repo_id, .. }) if repo_id == fixture.repo_id
    ));
    assert_eq!(
        fixture
            .repo
            .repo_catalog_membership_record(fixture.repo_id)?
            .map(|record| record.state()),
        Some(RepoCatalogMembershipState::Normal)
    );
    assert_eq!(
        fixture
            .repo
            .catalog_membership_runtime()
            .issue(fixture.repo_id),
        Err(CatalogMembershipError::NotMember(fixture.repo_id))
    );

    std::fs::remove_file(marker)?;
    let committed = commit_creation(&fixture.repo, &prepared)?;
    fixture
        .repo
        .catalog_membership_runtime()
        .revalidate(committed.membership())?;
    Ok(())
}

#[test]
fn post_replace_remove_failure_never_reopens_old_token() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let create = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    commit_creation(&fixture.repo, &create)?;
    let prepared = fixture
        .repo
        .prepare_repo_removal_membership(fixture.repo_id, Uuid::new_v4())?;
    let old = prepared.membership().clone();
    let store = store::RepoCatalogStore::open(fixture.repo.ledger_dir())?;
    let marker = store.post_replace_failure_marker();
    std::fs::write(&marker, b"inject")?;

    assert!(matches!(
        commit_removal(&fixture.repo, &prepared),
        Err(RepoCatalogError::CutOutcomeUnknown { repo_id, .. }) if repo_id == fixture.repo_id
    ));
    assert_eq!(
        fixture
            .repo
            .repo_catalog_membership_record(fixture.repo_id)?
            .map(|record| record.state()),
        Some(RepoCatalogMembershipState::Removed)
    );
    assert!(
        fixture
            .repo
            .catalog_membership_runtime()
            .revalidate(&old)
            .is_err()
    );

    std::fs::remove_file(marker)?;
    commit_removal(&fixture.repo, &prepared)?;
    assert!(
        fixture
            .repo
            .catalog_membership_runtime()
            .revalidate(&old)
            .is_err()
    );
    Ok(())
}

#[test]
fn pre_replace_remove_failure_restores_only_a_fresh_generation() -> anyhow::Result<()> {
    let fixture = fixture()?;
    let create = fixture
        .repo
        .prepare_repo_creation_membership(fixture.repo_id, Uuid::new_v4())?;
    commit_creation(&fixture.repo, &create)?;
    let prepared = fixture
        .repo
        .prepare_repo_removal_membership(fixture.repo_id, Uuid::new_v4())?;
    let old = prepared.membership().clone();
    let store = store::RepoCatalogStore::open(fixture.repo.ledger_dir())?;
    let marker = store.pre_replace_failure_marker();
    std::fs::write(&marker, b"inject")?;

    assert!(matches!(
        commit_removal(&fixture.repo, &prepared),
        Err(RepoCatalogError::PublishFailed { repo_id, phase: "before_replace", .. })
            if repo_id == fixture.repo_id
    ));
    assert_eq!(
        fixture
            .repo
            .repo_catalog_membership_record(fixture.repo_id)?
            .map(|record| record.state()),
        Some(RepoCatalogMembershipState::Normal)
    );
    assert!(
        fixture
            .repo
            .catalog_membership_runtime()
            .revalidate(&old)
            .is_err()
    );
    let fresh = fixture
        .repo
        .catalog_membership_runtime()
        .issue(fixture.repo_id)?;
    assert!(fresh.generation() > old.generation());
    std::fs::remove_file(marker)?;
    Ok(())
}

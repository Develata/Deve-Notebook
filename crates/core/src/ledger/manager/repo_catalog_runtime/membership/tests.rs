//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator

use super::*;

#[test]
fn seed_is_exact_and_single_use() {
    let runtime = CatalogMembershipRuntime::isolated();
    let repo = RepoId::new_v4();
    assert_eq!(
        runtime.seed([repo, repo]),
        Err(CatalogMembershipError::DuplicateSeed(repo))
    );
    runtime.seed([repo]).expect("first exact seed");
    runtime.seed([repo]).expect("same seed is idempotent");
    assert_eq!(
        runtime.seed([repo, RepoId::new_v4()]),
        Err(CatalogMembershipError::SeedDrift)
    );
}

#[test]
fn unrelated_repo_mutation_does_not_invalidate_token() {
    let runtime = CatalogMembershipRuntime::isolated();
    let first = RepoId::new_v4();
    let second = RepoId::new_v4();
    runtime.seed([first, second]).expect("seed");
    let first_token = runtime.issue(first).expect("first token");
    let second_token = runtime.issue(second).expect("second token");

    let revocation = runtime.begin_removal(&second_token).expect("begin revoke");
    runtime
        .finalize_removed(&revocation)
        .expect("finalize second");

    runtime
        .revalidate(&first_token)
        .expect("first remains exact");
}

#[test]
fn revoke_is_exact_and_readmission_rotates_generation() {
    let runtime = CatalogMembershipRuntime::isolated();
    let repo = RepoId::new_v4();
    runtime.seed([repo]).expect("seed");
    let old = runtime.issue(repo).expect("token");
    let revocation = runtime.begin_removal(&old).expect("begin revoke");
    runtime
        .finalize_removed(&revocation)
        .expect("finalize revoke");
    assert!(matches!(
        runtime.revalidate(&old),
        Err(CatalogMembershipError::Stale { repo_id, .. }) if repo_id == repo
    ));
    assert_eq!(
        runtime.issue(repo),
        Err(CatalogMembershipError::NotMember(repo))
    );

    let admitted = runtime.admit_created(repo).expect("readmit");
    assert!(admitted.generation().get() > old.generation().get() + 1);
    runtime.revalidate(&admitted).expect("new token exact");
    assert!(runtime.revalidate(&old).is_err());
}

#[test]
fn a_new_runtime_rejects_old_process_token() {
    let repo = RepoId::new_v4();
    let first = CatalogMembershipRuntime::isolated();
    first.seed([repo]).expect("first seed");
    let token = first.issue(repo).expect("first token");
    let rebuilt = CatalogMembershipRuntime::isolated();
    rebuilt.seed([repo]).expect("rebuilt seed");

    assert_eq!(
        rebuilt.revalidate(&token),
        Err(CatalogMembershipError::RuntimeMismatch(repo))
    );
    first
        .revalidate(&token)
        .expect("original runtime remains exact");
}

//! plan_ref:
//!   - 03_storage/repair#remote-import-cleanup-repair

use super::*;

fn fixture() -> anyhow::Result<(tempfile::TempDir, RepoId, RemoteImportArtifactRoot)> {
    let temp = tempfile::tempdir()?;
    let ledger = std::fs::canonicalize(temp.path())?.join("ledger");
    std::fs::create_dir(&ledger)?;
    let repo_id = RepoId::new_v4();
    let root = RemoteImportArtifactRoot::open(&ledger, repo_id)?;
    Ok((temp, repo_id, root))
}

#[test]
fn repo_cleanup_quarantines_then_deletes_the_whole_root() -> anyhow::Result<()> {
    let (_temp, repo_id, root) = fixture()?;
    std::fs::create_dir_all(root.path.join("session/blobs"))?;
    std::fs::write(root.path.join("session/blobs/payload"), b"captured")?;
    let plan = root.seal_repo_removal(repo_id)?;
    let prepared = RemoteImportArtifactRoot::initial_repo_removal_checkpoint();

    let quarantined = RemoteImportArtifactRoot::advance_repo_removal(&plan, &prepared)?;
    assert!(!root.path.exists());
    assert!(!quarantined.is_complete());
    let deleted = RemoteImportArtifactRoot::advance_repo_removal(&plan, &quarantined)?;
    assert!(deleted.is_complete());
    assert_eq!(
        RemoteImportArtifactRoot::advance_repo_removal(&plan, &deleted)?,
        deleted
    );
    Ok(())
}

#[test]
fn mutation_before_quarantine_checkpoint_is_reconstructed() -> anyhow::Result<()> {
    let (_temp, repo_id, root) = fixture()?;
    std::fs::write(root.path.join("payload"), b"captured")?;
    let plan = root.seal_repo_removal(repo_id)?;
    let prepared = RemoteImportArtifactRoot::initial_repo_removal_checkpoint();
    let expected = RemoteImportArtifactRoot::advance_repo_removal(&plan, &prepared)?;
    assert_eq!(
        RemoteImportArtifactRoot::advance_repo_removal(&plan, &prepared)?,
        expected
    );
    Ok(())
}

#[test]
fn compact_plan_does_not_embed_the_bounded_tree() -> anyhow::Result<()> {
    let (_temp, repo_id, root) = fixture()?;
    for index in 0..2_048 {
        std::fs::write(root.path.join(format!("blob-{index:04}")), b"x")?;
    }
    let plan = root.seal_repo_removal(repo_id)?;
    assert!(serde_json::to_vec(&plan)?.len() < 4 * 1024);
    assert!(std::fs::metadata(plan.inventory.path())?.len() > 64 * 1024);
    assert!(RemoteImportArtifactRoot::revalidate_repo_removal(&plan)?);
    Ok(())
}

#[test]
fn invalidation_preserves_payload_and_the_single_owner_slot() -> anyhow::Result<()> {
    let (_temp, repo_id, root) = fixture()?;
    let payload = root.path.join("payload");
    std::fs::write(&payload, b"captured")?;
    let first = root.seal_repo_removal(repo_id)?;
    RemoteImportArtifactRoot::invalidate_repo_removal(&first)?;
    RemoteImportArtifactRoot::invalidate_repo_removal(&first)?;
    assert_eq!(std::fs::read(&payload)?, b"captured");
    assert!(first.inventory.path().exists());
    assert!(!RemoteImportArtifactRoot::revalidate_repo_removal(&first)?);

    let second = root.seal_repo_removal(repo_id)?;
    assert_ne!(first.logical_epoch, second.logical_epoch);
    match RemoteImportArtifactRoot::revalidate_repo_removal(&first) {
        Ok(false) => {}
        // Unix inode numbers may be reused after the invalidated sidecar is
        // atomically replaced. The digest check must still reject that ABA as
        // changed content; it is a fail-closed stale-plan result, not a match.
        Err(RemoteImportError::UnsafeArtifactRoot(detail))
            if detail == "repo artifact removal inventory content changed" => {}
        Ok(true) => anyhow::bail!("invalidated removal plan became exact after a new seal"),
        Err(error) => return Err(error.into()),
    }
    assert!(RemoteImportArtifactRoot::revalidate_repo_removal(&second)?);
    let sidecars = std::fs::read_dir(&root.path)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() == REMOVAL_PLAN_NAME)
        .count();
    assert_eq!(sidecars, 1);
    Ok(())
}

#[test]
fn recycled_inventory_identity_still_rejects_the_stale_plan_digest() -> anyhow::Result<()> {
    let (_temp, repo_id, root) = fixture()?;
    std::fs::write(root.path.join("payload"), b"captured")?;
    let first = root.seal_repo_removal(repo_id)?;
    RemoteImportArtifactRoot::invalidate_repo_removal(&first)?;
    let second = root.seal_repo_removal(repo_id)?;

    // Model a Unix (device, inode) ABA: the stale plan observes the current
    // sidecar's numeric identity while retaining its old epoch and digest.
    let mut recycled_identity = first.clone();
    recycled_identity.inventory = second.inventory.clone();
    assert!(matches!(
        RemoteImportArtifactRoot::revalidate_repo_removal(&recycled_identity),
        Err(RemoteImportError::UnsafeArtifactRoot(detail))
            if detail == "repo artifact removal inventory content changed"
    ));
    assert!(RemoteImportArtifactRoot::revalidate_repo_removal(&second)?);
    Ok(())
}

#[test]
fn partial_payload_deletion_resumes_from_an_identity_exact_subset() -> anyhow::Result<()> {
    let (_temp, repo_id, root) = fixture()?;
    std::fs::write(root.path.join("first"), b"captured")?;
    std::fs::write(root.path.join("second"), b"captured")?;
    let plan = root.seal_repo_removal(repo_id)?;
    let prepared = RemoteImportArtifactRoot::initial_repo_removal_checkpoint();
    let quarantined = RemoteImportArtifactRoot::advance_repo_removal(&plan, &prepared)?;
    let moved_root = match &quarantined.state {
        RemoteImportArtifactRemovalCheckpointState::RootQuarantined { root } => root.path(),
        _ => unreachable!("first advance quarantines the root"),
    };
    std::fs::remove_file(moved_root.join("first"))?;

    let deleted = RemoteImportArtifactRoot::advance_repo_removal(&plan, &quarantined)?;

    assert!(deleted.is_complete());
    assert!(!moved_root.exists());
    Ok(())
}

#[test]
fn empty_quarantine_after_sidecar_cut_finishes_exact_root_deletion() -> anyhow::Result<()> {
    let (_temp, repo_id, root) = fixture()?;
    std::fs::write(root.path.join("payload"), b"captured")?;
    let plan = root.seal_repo_removal(repo_id)?;
    let prepared = RemoteImportArtifactRoot::initial_repo_removal_checkpoint();
    let quarantined = RemoteImportArtifactRoot::advance_repo_removal(&plan, &prepared)?;
    let moved_root = match &quarantined.state {
        RemoteImportArtifactRemovalCheckpointState::RootQuarantined { root } => root.path(),
        _ => unreachable!("first advance quarantines the root"),
    };
    std::fs::remove_file(moved_root.join("payload"))?;
    std::fs::remove_file(moved_root.join(REMOVAL_PLAN_NAME))?;

    let deleted = RemoteImportArtifactRoot::advance_repo_removal(&plan, &quarantined)?;

    assert!(deleted.is_complete());
    assert!(!moved_root.exists());
    Ok(())
}

#[test]
fn changed_quarantined_tree_is_not_deleted() -> anyhow::Result<()> {
    let (_temp, repo_id, root) = fixture()?;
    std::fs::write(root.path.join("payload"), b"captured")?;
    let plan = root.seal_repo_removal(repo_id)?;
    let prepared = RemoteImportArtifactRoot::initial_repo_removal_checkpoint();
    let quarantined = RemoteImportArtifactRoot::advance_repo_removal(&plan, &prepared)?;
    let moved_root = match &quarantined.state {
        RemoteImportArtifactRemovalCheckpointState::RootQuarantined { root } => root.path(),
        _ => unreachable!("first advance quarantines the root"),
    };
    let injected = moved_root.join("injected");
    std::fs::write(&injected, b"foreign")?;
    assert!(RemoteImportArtifactRoot::advance_repo_removal(&plan, &quarantined).is_err());
    assert_eq!(std::fs::read(injected)?, b"foreign");
    Ok(())
}

#[test]
fn unsealed_entry_blocks_quarantine_and_is_preserved() -> anyhow::Result<()> {
    let (_temp, repo_id, root) = fixture()?;
    std::fs::write(root.path.join("payload"), b"captured")?;
    let plan = root.seal_repo_removal(repo_id)?;
    let unexpected = root.path.join("unexpected");
    std::fs::write(&unexpected, b"external")?;
    assert!(!RemoteImportArtifactRoot::revalidate_repo_removal(&plan)?);
    let checkpoint = RemoteImportArtifactRoot::initial_repo_removal_checkpoint();
    assert!(RemoteImportArtifactRoot::advance_repo_removal(&plan, &checkpoint).is_err());
    assert_eq!(std::fs::read(unexpected)?, b"external");
    Ok(())
}

#[test]
fn tampered_sidecar_blocks_quarantine_without_deleting_payload() -> anyhow::Result<()> {
    let (_temp, repo_id, root) = fixture()?;
    let payload = root.path.join("payload");
    std::fs::write(&payload, b"captured")?;
    let plan = root.seal_repo_removal(repo_id)?;
    std::fs::write(plan.inventory.path(), b"{}")?;
    let checkpoint = RemoteImportArtifactRoot::initial_repo_removal_checkpoint();
    assert!(RemoteImportArtifactRoot::advance_repo_removal(&plan, &checkpoint).is_err());
    assert_eq!(std::fs::read(payload)?, b"captured");
    Ok(())
}

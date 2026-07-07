use super::super::dispatch::dispatch_batch;
use super::super::dispatch_test_support::{event_for, new_sync};
use crate::codec;
use crate::ledger::{
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
};

#[test]
fn dispatch_batch_fails_closed_on_dir_change_resolution_error() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let docs = repo_root.join("docs");
    std::fs::create_dir_all(&docs)?;
    repo.run_on_local_repo(&repo_name, |db| {
        let write = db.begin_write()?;
        {
            let mut table = write.open_table(REPO_METADATA)?;
            let version = codec::encode(&REDB_SCHEMA_VERSION)?;
            table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
            table.insert(&REPO_INFO_METADATA_KEY, b"not-postcard".as_slice())?;
        }
        write.commit()?;
        Ok::<_, anyhow::Error>(())
    })?;

    let err = dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(docs)],
        None,
    )
    .expect_err("dir change resolution must fail closed");

    assert!(err.to_string().contains("Failed to handle dir change"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn dispatch_batch_fails_closed_on_unstatable_dir_event() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, _repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let blocked = repo_root.join("blocked");
    std::fs::create_dir_all(&blocked)?;
    let original = std::fs::metadata(&blocked)?.permissions();
    let mut blocked_perms = original.clone();
    blocked_perms.set_mode(0o000);
    std::fs::set_permissions(&blocked, blocked_perms)?;

    let result = dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(blocked.clone())],
        None,
    );
    std::fs::set_permissions(&blocked, original)?;
    let err = result.expect_err("unstatable dir event must fail closed");
    let detail = err.to_string();
    assert!(
        detail.contains("Failed to classify watcher event")
            || detail.contains("Failed to handle dir change")
            || detail.contains("Permission denied"),
        "unexpected error: {detail}"
    );
    Ok(())
}

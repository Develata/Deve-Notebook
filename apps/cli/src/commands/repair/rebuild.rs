use anyhow::Result;
use deve_core::sync::SyncManager;

pub(super) fn materialize_repos(
    sync_manager: &SyncManager,
    repo_names: &[String],
) -> Result<usize> {
    for repo_name in repo_names {
        sync_manager.materialize_local_repo(repo_name)?;
        println!("repair: rebuilt projection for repo {}", repo_name);
    }
    Ok(repo_names.len())
}

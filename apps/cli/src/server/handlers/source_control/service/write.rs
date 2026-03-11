use super::super::errors::{self, ScOp};
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::CommitInfo;

pub fn stage_pending(
    repo: &dyn Repository,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> super::ScResult<String> {
    let entries = repo
        .list_pending_fs_in_repo(selector)
        .map_err(|e| errors::map_repo_error(ScOp::StagePending(target.path.clone()), e))?;
    let resolved = super::resolve_target(&entries, target);
    let path = resolved.path.clone();
    for related_target in super::related_targets(&entries, &resolved) {
        repo.stage_pending_in_repo(selector, &related_target)
            .map_err(|e| errors::map_repo_error(ScOp::StagePending(path.clone()), e))?;
    }
    Ok(path)
}

pub fn stage_pending_many(
    repo: &dyn Repository,
    selector: &RepoSelector,
    targets: Vec<ScPathTarget>,
) -> super::ScResult<Vec<String>> {
    let entries = repo
        .list_pending_fs_in_repo(selector)
        .map_err(|e| errors::map_repo_error(ScOp::ListPending, e))?;
    let resolved_targets = super::resolve_targets(&entries, targets);
    let visible_paths: Vec<_> = resolved_targets
        .iter()
        .map(|target| target.path.clone())
        .collect();
    for target in &resolved_targets {
        let path = target.path.clone();
        for related_target in super::related_targets(&entries, target) {
            repo.stage_pending_in_repo(selector, &related_target)
                .map_err(|e| errors::map_repo_error(ScOp::StagePending(path.clone()), e))?;
        }
    }
    Ok(visible_paths)
}

pub fn discard_pending(
    repo: &dyn Repository,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> super::ScResult<String> {
    let entries = repo
        .list_pending_fs_in_repo(selector)
        .map_err(|e| errors::map_repo_error(ScOp::DiscardPending(target.path.clone()), e))?;
    let resolved = super::resolve_target(&entries, target);
    let path = resolved.path.clone();
    repo.discard_pending_in_repo(selector, &resolved)
        .map_err(|e| errors::map_repo_error(ScOp::DiscardPending(path.clone()), e))?;
    Ok(path)
}

pub fn unstage_file(
    repo: &dyn Repository,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> super::ScResult<String> {
    let entries = repo
        .list_changes_in_repo(selector)
        .map_err(|e| errors::map_repo_error(ScOp::Unstage(target.path.clone()), e))?;
    let resolved = super::resolve_target(&entries, target);
    let path = resolved.path.clone();
    for related_target in super::related_targets(&entries, &resolved) {
        repo.unstage_file_in_repo(selector, &related_target)
            .map_err(|e| errors::map_repo_error(ScOp::Unstage(path.clone()), e))?;
    }
    Ok(path)
}

pub fn unstage_many(
    repo: &dyn Repository,
    selector: &RepoSelector,
    targets: Vec<ScPathTarget>,
) -> super::ScResult<Vec<String>> {
    let entries = repo
        .list_changes_in_repo(selector)
        .map_err(|e| errors::map_repo_error(ScOp::ListChanges, e))?;
    let resolved_targets = super::resolve_targets(&entries, targets);
    let visible_paths: Vec<_> = resolved_targets
        .iter()
        .map(|target| target.path.clone())
        .collect();
    for target in &resolved_targets {
        let path = target.path.clone();
        for related_target in super::related_targets(&entries, target) {
            repo.unstage_file_in_repo(selector, &related_target)
                .map_err(|e| errors::map_repo_error(ScOp::Unstage(path.clone()), e))?;
        }
    }
    Ok(visible_paths)
}

pub fn commit_staged(
    repo: &dyn Repository,
    selector: &RepoSelector,
    message: &str,
) -> super::ScResult<CommitInfo> {
    repo.commit_staged_in_repo(selector, message)
        .map_err(|e| errors::map_repo_error(ScOp::Commit, e))
}

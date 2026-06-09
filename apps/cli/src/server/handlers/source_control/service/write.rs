//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/index#repo-runtime-layout

use super::super::errors::{self, ScOp};
use deve_core::config::GitBridgeMode;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{CommitInfo, SourceControlApi};

pub fn stage_pending(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> super::ScResult<String> {
    let entries = repo
        .list_pending_fs_in_repo(selector)
        .map_err(|e| errors::map_repo_error(ScOp::StagePending(target.path.clone()), e))?;
    let resolved = super::resolve_target(&entries, target)?;
    let path = resolved.path.clone();
    for related_target in super::related_targets(&entries, &resolved)? {
        repo.stage_pending_in_repo(selector, &related_target)
            .map_err(|e| errors::map_repo_error(ScOp::StagePending(path.clone()), e))?;
    }
    Ok(path)
}

pub fn stage_pending_many(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
    targets: Vec<ScPathTarget>,
) -> super::ScResult<Vec<String>> {
    let entries = repo
        .list_pending_fs_in_repo(selector)
        .map_err(|e| errors::map_repo_error(ScOp::ListPending, e))?;
    let resolved_targets = super::resolve_targets(&entries, targets)?;
    let visible_paths: Vec<_> = resolved_targets
        .iter()
        .map(|target| target.path.clone())
        .collect();
    for target in &resolved_targets {
        let path = target.path.clone();
        for related_target in super::related_targets(&entries, target)? {
            repo.stage_pending_in_repo(selector, &related_target)
                .map_err(|e| errors::map_repo_error(ScOp::StagePending(path.clone()), e))?;
        }
    }
    Ok(visible_paths)
}

pub fn discard_pending(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> super::ScResult<String> {
    let entries = repo
        .list_pending_fs_in_repo(selector)
        .map_err(|e| errors::map_repo_error(ScOp::DiscardPending(target.path.clone()), e))?;
    let resolved = super::resolve_target(&entries, target)?;
    let path = resolved.path.clone();
    repo.discard_pending_in_repo(selector, &resolved)
        .map_err(|e| errors::map_repo_error(ScOp::DiscardPending(path.clone()), e))?;
    Ok(path)
}

pub fn unstage_file(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> super::ScResult<String> {
    let entries = repo
        .list_changes_in_repo(selector)
        .map_err(|e| errors::map_repo_error(ScOp::Unstage(target.path.clone()), e))?;
    let resolved = super::resolve_target(&entries, target)?;
    let path = resolved.path.clone();
    for related_target in super::related_targets(&entries, &resolved)? {
        repo.unstage_file_in_repo(selector, &related_target)
            .map_err(|e| errors::map_repo_error(ScOp::Unstage(path.clone()), e))?;
    }
    Ok(path)
}

pub fn unstage_many(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
    targets: Vec<ScPathTarget>,
) -> super::ScResult<Vec<String>> {
    let entries = repo
        .list_changes_in_repo(selector)
        .map_err(|e| errors::map_repo_error(ScOp::ListChanges, e))?;
    let resolved_targets = super::resolve_targets(&entries, targets)?;
    let visible_paths: Vec<_> = resolved_targets
        .iter()
        .map(|target| target.path.clone())
        .collect();
    for target in &resolved_targets {
        let path = target.path.clone();
        for related_target in super::related_targets(&entries, target)? {
            repo.unstage_file_in_repo(selector, &related_target)
                .map_err(|e| errors::map_repo_error(ScOp::Unstage(path.clone()), e))?;
        }
    }
    Ok(visible_paths)
}

pub fn commit_staged_with_git_bridge(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
    message: &str,
    git_bridge: GitBridgeMode,
) -> super::ScResult<CommitInfo> {
    repo.commit_staged_in_repo_with_git_bridge(selector, message, git_bridge)
        .map_err(|e| errors::map_repo_error(ScOp::Commit, e))
}

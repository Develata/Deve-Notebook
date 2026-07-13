//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::hooks::use_core::source_control_notice::is_local_command_notice;
use crate::runtime::source_control_client::diff_cache::{projection_scope_key, put_projection};
use crate::runtime::source_control_client::diff_session::{DiffSessionWire, MergeConflictSession};
use deve_core::protocol::ServerMessage;
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::effects_sc_apply::apply_doc_diff;
use super::super::effects_sc_state::{
    changes_list_matches_request, commit_diff_matches_request, commit_history_matches_request,
    doc_diff_matches_request,
};
use super::ScMessageContext;

pub(crate) fn handle_sc_list_message(
    msg: &ServerMessage,
    ctx: &ScMessageContext<'_>,
    active_scope_nonce: u64,
) -> bool {
    match msg {
        ServerMessage::ChangesList {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            staged,
            unstaged,
            confirmed,
        } => {
            if !ctx.in_scope(repo_id, branch) {
                return true;
            }
            if !changes_list_matches_request(
                request_id,
                ctx.changes_request_id.get_untracked(),
                *scope_nonce,
                active_scope_nonce,
            ) {
                return true;
            }
            clear_non_local_notice(ctx);
            ctx.set_changes_request_id.set(None);
            ctx.set_staged.set(staged.clone());
            ctx.set_unstaged.set(unstaged.clone());
            ctx.set_confirmed.set(confirmed.clone());
            true
        }
        ServerMessage::CommitHistory {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            commits,
        } => {
            if !ctx.in_scope(repo_id, branch) {
                return true;
            }
            if !commit_history_matches_request(
                request_id,
                ctx.commit_history_request_id.get_untracked(),
                *scope_nonce,
                active_scope_nonce,
            ) {
                return true;
            }
            clear_non_local_notice(ctx);
            ctx.set_commit_history_request_id.set(None);
            ctx.set_history.set(commits.clone());
            true
        }
        ServerMessage::DocDiff {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            doc_id,
            path,
            projection,
        } => {
            if !ctx.in_scope(repo_id, branch) {
                return true;
            }
            let matches_doc_request = doc_diff_matches_request(
                request_id,
                ctx.doc_diff_request_id.get_untracked(),
                *scope_nonce,
                active_scope_nonce,
            );
            let matches_commit_file_request = doc_diff_matches_request(
                request_id,
                ctx.commit_diff_request_id.get_untracked(),
                *scope_nonce,
                active_scope_nonce,
            );
            if !matches_doc_request && !matches_commit_file_request {
                return true;
            }
            let active_request = ctx
                .diff
                .get_untracked()
                .is_some_and(|current| current.matches_pending_request(request_id.as_deref()));
            if !active_request {
                if matches_doc_request {
                    ctx.set_doc_diff_request_id.set(None);
                }
                if matches_commit_file_request {
                    ctx.set_commit_diff_request_id.set(None);
                }
                return true;
            }
            clear_non_local_notice(ctx);
            if matches_doc_request {
                ctx.set_doc_diff_request_id.set(None);
            }
            if matches_commit_file_request {
                ctx.set_commit_diff_request_id.set(None);
            }
            if is_merge_conflict_diff_fallback(ctx.diff.get_untracked(), request_id, *doc_id, path)
            {
                return true;
            }
            let cache_key = ctx
                .diff
                .get_untracked()
                .filter(|current| current.path == *path)
                .and_then(|current| current.cache_key);
            if let Some(cache_key) = cache_key.as_ref() {
                let repo_key = repo_id.as_ref().map(ToString::to_string);
                let scope_key = projection_scope_key(
                    repo_key.as_deref(),
                    branch.as_ref(),
                    scope_nonce.unwrap_or(active_scope_nonce),
                );
                put_projection(&scope_key, cache_key.clone(), projection.clone());
            }
            apply_doc_diff(
                request_id.as_deref(),
                *doc_id,
                path,
                projection,
                cache_key,
                ctx.set_diff,
            );
            true
        }
        ServerMessage::MergeConflict {
            repo_id,
            branch,
            scope_nonce,
            doc_id,
            path,
            projection,
            result_content,
            actions,
            ..
        } => {
            if !ctx.in_scope(repo_id, branch) || *scope_nonce != Some(active_scope_nonce) {
                return true;
            }
            clear_non_local_notice(ctx);
            leptos::logging::log!("收到合并冲突: {} ({} actions)", path, actions.len());
            ctx.set_diff.set(Some(
                DiffSessionWire::from_projection(path.clone(), projection.clone())
                    .with_doc_id(Some(*doc_id))
                    .with_merge_conflict(MergeConflictSession {
                        doc_id: *doc_id,
                        result_content: result_content.clone(),
                        actions: actions.clone(),
                    }),
            ));
            true
        }
        ServerMessage::CommitDiffResult {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            files,
        } => {
            if !ctx.in_scope(repo_id, branch) {
                return true;
            }
            if !commit_diff_matches_request(
                request_id,
                ctx.commit_diff_request_id.get_untracked(),
                *scope_nonce,
                active_scope_nonce,
            ) {
                return true;
            }
            clear_non_local_notice(ctx);
            ctx.set_commit_diff_request_id.set(None);
            leptos::logging::log!("收到提交差异: {} 个文件变更", files.len());
            ctx.set_commit_diff.set(files.clone());
            true
        }
        ServerMessage::DiffProjectionResult {
            request_id,
            revision,
            repo_id,
            branch,
            scope_nonce,
            projection,
        } => {
            if !ctx.in_scope(&Some(*repo_id), branch) || scope_nonce.get() != active_scope_nonce {
                return true;
            }
            ctx.set_diff.update(|current| {
                if let Some(current) = current
                    && current.accepts_result(request_id, *revision)
                {
                    current.install_projection(projection.clone());
                }
            });
            true
        }
        ServerMessage::DiffProjectionError {
            request_id,
            revision,
            repo_id,
            branch,
            scope_nonce,
            error,
        } => {
            if !ctx.in_scope(&Some(*repo_id), branch) || scope_nonce.get() != active_scope_nonce {
                return true;
            }
            if *revision == 0 {
                if ctx.doc_diff_request_id.get_untracked().as_deref() == Some(request_id) {
                    ctx.set_doc_diff_request_id.set(None);
                }
                if ctx.commit_diff_request_id.get_untracked().as_deref() == Some(request_id) {
                    ctx.set_commit_diff_request_id.set(None);
                }
            }
            ctx.set_diff.update(|current| {
                if let Some(current) = current
                    && current.accepts_error(request_id, *revision)
                {
                    current.install_error(error.clone());
                }
            });
            true
        }
        _ => false,
    }
}

fn clear_non_local_notice(ctx: &ScMessageContext<'_>) {
    ctx.set_notice.update(|notice| {
        if notice.as_ref().is_some_and(is_local_command_notice) {
            return;
        }
        *notice = None;
    });
}

fn is_merge_conflict_diff_fallback(
    current: Option<DiffSessionWire>,
    request_id: &Option<String>,
    doc_id: Option<deve_core::models::DocId>,
    path: &str,
) -> bool {
    request_id.is_none()
        && current.is_some_and(|diff| {
            diff.path == path
                && diff
                    .merge_conflict
                    .is_some_and(|conflict| Some(conflict.doc_id) == doc_id)
        })
}

#[cfg(test)]
mod tests {
    use super::is_merge_conflict_diff_fallback;
    use crate::runtime::source_control_client::diff_session::{
        DiffSessionWire, MergeConflictSession,
    };
    use deve_core::models::DocId;
    use deve_core::protocol::MergeConflictAction;

    #[test]
    fn doc_diff_fallback_does_not_overwrite_merge_conflict_session() {
        let doc_id = DocId::new();
        let diff = DiffSessionWire::new("notes/a.md".into(), "local".into(), "remote".into())
            .with_doc_id(Some(doc_id))
            .with_merge_conflict(MergeConflictSession {
                doc_id,
                result_content: "base".into(),
                actions: vec![MergeConflictAction::AcceptBoth],
            });

        assert!(is_merge_conflict_diff_fallback(
            Some(diff),
            &None,
            Some(doc_id),
            "notes/a.md",
        ));
    }

    #[test]
    fn requested_doc_diff_still_replaces_plain_diff_view() {
        let doc_id = DocId::new();
        let diff = DiffSessionWire::new("notes/a.md".into(), "old".into(), "new".into())
            .with_doc_id(Some(doc_id));

        assert!(!is_merge_conflict_diff_fallback(
            Some(diff),
            &Some("doc-req-1".into()),
            Some(doc_id),
            "notes/a.md",
        ));
    }
}

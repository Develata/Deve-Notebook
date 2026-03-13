// apps/web/src/editor/sync/snapshot.rs
//! Snapshot 消息处理: 接收文档快照并渐进式应用 delta ops

use super::context::SyncContext;
use super::scope::matches_scope;
use super::snapshot_finish::{LoadFinish, emit_stats, finalize_load, now_ms};
use crate::editor::ffi::{applyRemoteContent, applyRemoteOpsBatch};
use crate::editor::prefetch::{PrefetchConfig, apply_ops_in_batches};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ConfirmedOp;
use leptos::prelude::*;

type BatchHandler = std::rc::Rc<dyn Fn(&[ConfirmedOp])>;
type ProgressHandler = std::rc::Rc<dyn Fn(usize, usize, f64)>;

/// 处理 ServerMessage::Snapshot
///
/// # Pre-conditions
/// - `msg_doc_id` 已验证匹配 ctx.doc_id
///
/// # Post-conditions
/// - 编辑器内容更新为快照 + delta ops
/// - local_version 推进到最新 seq
pub(super) fn handle_snapshot(
    ctx: &SyncContext,
    repo_id: RepoId,
    branch: Option<PeerId>,
    request_id: u64,
    new_content: String,
    base_seq: u64,
    version: u64,
    delta_ops: Vec<ConfirmedOp>,
) {
    let load_start = now_ms();
    let open_request_id = ctx.open_request_id;
    let current_repo_id = ctx.current_repo_id;
    let pending_repo_switch = ctx.pending_repo_switch;
    let active_branch = ctx.active_branch;
    let pending_branch_switch = ctx.pending_branch_switch;

    leptos::logging::log!(
        "Received Snapshot: {} chars, Base: {}, Ver: {}, Pending: {}",
        new_content.len(),
        base_seq,
        version,
        delta_ops.len()
    );

    emit_stats(ctx.on_stats, &new_content);
    applyRemoteContent(&new_content);
    ctx.set_content.set(new_content);
    ctx.set_local_version.set(base_seq);
    ctx.set_playback_version.set(base_seq);
    ctx.set_load_state.set("partial".to_string());
    ctx.set_load_progress.set((0, delta_ops.len()));
    ctx.set_load_eta_ms.set(0);

    if delta_ops.is_empty() {
        finalize_load(ctx, version, load_start);
        return;
    }

    let apply_batch = build_apply_batch(
        ctx.set_local_version,
        ctx.set_history,
        open_request_id,
        current_repo_id,
        pending_repo_switch,
        active_branch,
        pending_branch_switch,
        repo_id,
        branch.clone(),
        request_id,
    );
    let on_progress = build_progress_handler(ctx.set_load_progress, ctx.set_load_eta_ms);
    let finish = LoadFinish::from_ctx(ctx, version, load_start, request_id);
    let on_done = std::rc::Rc::new(move || {
        if snapshot_request_matches(
            open_request_id.get_untracked(),
            request_id,
            current_repo_id.get_untracked(),
            pending_repo_switch.get_untracked(),
            active_branch.get_untracked(),
            pending_branch_switch.get_untracked(),
            repo_id,
            branch.clone(),
        ) {
            finish.clone().complete();
        }
    });

    apply_ops_in_batches(
        delta_ops,
        PrefetchConfig {
            target_ms: 8.0,
            initial_batch: 16,
            max_batch: 256,
        },
        apply_batch,
        on_progress,
        on_done,
    );
}

fn build_apply_batch(
    set_local_version: WriteSignal<u64>,
    set_history: WriteSignal<Vec<(u64, deve_core::models::Op)>>,
    open_request_id: ReadSignal<u64>,
    current_repo_id: ReadSignal<Option<String>>,
    pending_repo_switch: ReadSignal<Option<String>>,
    active_branch: ReadSignal<Option<PeerId>>,
    pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    repo_id: RepoId,
    branch: Option<PeerId>,
    request_id: u64,
) -> BatchHandler {
    std::rc::Rc::new(move |batch: &[ConfirmedOp]| {
        if !snapshot_request_matches(
            open_request_id.get_untracked(),
            request_id,
            current_repo_id.get_untracked(),
            pending_repo_switch.get_untracked(),
            active_branch.get_untracked(),
            pending_branch_switch.get_untracked(),
            repo_id,
            branch.clone(),
        ) {
            return;
        }
        let ops_only: Vec<_> = batch.iter().map(|entry| entry.op.clone()).collect();
        if let Ok(json) = serde_json::to_string(&ops_only) {
            applyRemoteOpsBatch(&json);
        }
        if let Some(entry) = batch.last() {
            set_local_version.set(entry.seq);
        }
        set_history.update(|history| {
            for entry in batch {
                history.push((entry.seq, entry.op.clone()));
            }
        });
    })
}

fn build_progress_handler(
    set_load_progress: WriteSignal<(usize, usize)>,
    set_load_eta_ms: WriteSignal<u64>,
) -> ProgressHandler {
    let elapsed_total = std::rc::Rc::new(std::cell::RefCell::new(0.0));
    std::rc::Rc::new(move |done: usize, total: usize, batch_ms: f64| {
        set_load_progress.set((done, total));
        *elapsed_total.borrow_mut() += batch_ms;
        if done > 0 {
            let per_op = *elapsed_total.borrow() / done as f64;
            let remaining = (total - done) as f64 * per_op;
            set_load_eta_ms.set(remaining as u64);
        }
    })
}

fn snapshot_request_matches(
    open_request_id: u64,
    request_id: u64,
    current_repo_id: Option<String>,
    pending_repo_switch: Option<String>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    repo_id: RepoId,
    branch: Option<PeerId>,
) -> bool {
    open_request_id == request_id
        && matches_scope(
            current_repo_id,
            pending_repo_switch,
            active_branch,
            pending_branch_switch,
            Some(repo_id),
            branch,
        )
}

#[cfg(test)]
mod tests {
    use super::snapshot_request_matches;
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::models::PeerId;

    #[test]
    fn snapshot_request_rejects_pending_repo_switch() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(!snapshot_request_matches(
            7,
            7,
            Some(repo_id.to_string()),
            Some("test".into()),
            None,
            None,
            repo_id,
            None,
        ));
    }

    #[test]
    fn snapshot_request_rejects_branch_mismatch_even_with_same_request_id() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(!snapshot_request_matches(
            7,
            7,
            Some(repo_id.to_string()),
            None,
            Some(PeerId::new("peer-a")),
            None,
            repo_id,
            Some(PeerId::new("peer-b")),
        ));
        assert!(!snapshot_request_matches(
            7,
            7,
            Some(repo_id.to_string()),
            None,
            Some(PeerId::new("peer-a")),
            Some(PendingBranchTarget::Local),
            repo_id,
            None,
        ));
        assert!(snapshot_request_matches(
            7,
            7,
            Some(repo_id.to_string()),
            None,
            None,
            None,
            repo_id,
            None,
        ));
    }
}

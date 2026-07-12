// apps/web/src/editor/sync/context.rs
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! SyncContext: 将同步参数打包为一个上下文结构体

use crate::api::WsService;
use crate::editor::EditorStats;
use crate::hooks::use_core::navigation::PendingNavigation;
use crate::runtime::document::pending::PendingLocalEdits;
use crate::runtime::domain::{
    EditorSyncFailure, EditorSyncFailureCode, LoadPhase, PendingBranchSwitch, PendingRepoSwitch,
};
use deve_core::models::{DocId, Op, PeerId};
use deve_core::protocol::ConfirmedOp;
use deve_core::security::{EncryptedOp, RepoKey};
use leptos::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(super) struct EditorSyncFailureSink {
    session_generation: Arc<AtomicU64>,
    ready_generation: Arc<AtomicU64>,
    open_request_id: ReadSignal<u64>,
    set_load_state: WriteSignal<LoadPhase>,
    set_load_progress: WriteSignal<(usize, usize)>,
    set_load_eta_ms: WriteSignal<u64>,
    set_editor_sync_failure: WriteSignal<Option<EditorSyncFailure>>,
}

impl EditorSyncFailureSink {
    pub(super) fn fail(&self, code: EditorSyncFailureCode) {
        lock_editor_projection();
        self.ready_generation.store(0, Ordering::Relaxed);
        self.set_load_state.set(LoadPhase::Error);
        self.set_load_progress.set((0, 0));
        self.set_load_eta_ms.set(0);
        self.set_editor_sync_failure
            .set(Some(EditorSyncFailure::new(
                code,
                self.session_generation.load(Ordering::Relaxed),
                self.open_request_id.get_untracked(),
            )));
    }
}

#[cfg(target_arch = "wasm32")]
fn lock_editor_projection() {
    crate::editor::ffi::set_read_only(true);
}

#[cfg(not(target_arch = "wasm32"))]
fn lock_editor_projection() {}

/// 同步消息处理所需的全部上下文
///
/// # Invariants
/// - `doc_id` 在整个编辑器会话中保持不变
/// - `client_id` 唯一标识当前客户端实例
/// - `repo_key` 仅在内存中持有，页面卸载时清除 (NEVER persisted)
/// - `buffered_encrypted_ops` 仅在 `repo_key` 未就绪时临时缓存加密推送，绝不静默丢弃
pub struct SyncContext<'a> {
    pub doc_id: DocId,
    pub client_id: Option<u64>,
    pub session_generation: Arc<AtomicU64>,
    pub ready_generation: Arc<AtomicU64>,
    pub buffered_live_ops: Arc<Mutex<Vec<ConfirmedOp>>>,
    pub buffered_encrypted_ops: Arc<Mutex<Vec<EncryptedOp>>>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    pub handshake_scope_nonce: ReadSignal<Option<u64>>,
    pub load_state: ReadSignal<LoadPhase>,
    pub is_spectator: Signal<bool>,
    pub handshake_ready: ReadSignal<bool>,
    pub open_request_id: ReadSignal<u64>,
    pub set_open_request_id: WriteSignal<u64>,
    pub ws: &'a WsService,
    // 内容信号
    pub content: ReadSignal<String>,
    pub set_content: WriteSignal<String>,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
    pub set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    // 版本追踪
    pub local_version: ReadSignal<u64>,
    pub set_local_version: WriteSignal<u64>,
    // 历史记录
    pub history: ReadSignal<Vec<(u64, Op)>>,
    pub set_history: WriteSignal<Vec<(u64, Op)>>,
    // 回放控制
    pub is_playback: ReadSignal<bool>,
    pub set_playback_version: WriteSignal<u64>,
    // 加载进度
    pub set_load_state: WriteSignal<LoadPhase>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub set_load_eta_ms: WriteSignal<u64>,
    pub set_editor_sync_failure: WriteSignal<Option<EditorSyncFailure>>,
    pub snapshot_reopen_attempted: ReadSignal<bool>,
    pub set_snapshot_reopen_attempted: WriteSignal<bool>,
    // 统计回调
    pub on_stats: Option<Callback<EditorStats>>,
    // E2EE: 仓库密钥 (RAM-only)
    pub repo_key: ReadSignal<Option<RepoKey>>,
    pub set_repo_key: WriteSignal<Option<RepoKey>>,
}

impl SyncContext<'_> {
    pub fn current_generation(&self) -> u64 {
        self.session_generation.load(Ordering::Relaxed)
    }

    pub fn is_generation_current(&self, expected_generation: u64) -> bool {
        self.current_generation() == expected_generation
    }

    pub fn mark_live_ready(&self, expected_generation: u64) {
        if self.is_generation_current(expected_generation) {
            self.ready_generation
                .store(expected_generation, Ordering::Relaxed);
        }
    }

    pub fn is_live_ready(&self) -> bool {
        let current_generation = self.current_generation();
        current_generation != 0
            && self.ready_generation.load(Ordering::Relaxed) == current_generation
    }

    pub fn fail_editor_sync(&self, code: EditorSyncFailureCode) {
        self.failure_sink().fail(code);
    }

    pub(super) fn failure_sink(&self) -> EditorSyncFailureSink {
        EditorSyncFailureSink {
            session_generation: self.session_generation.clone(),
            ready_generation: self.ready_generation.clone(),
            open_request_id: self.open_request_id,
            set_load_state: self.set_load_state,
            set_load_progress: self.set_load_progress,
            set_load_eta_ms: self.set_load_eta_ms,
            set_editor_sync_failure: self.set_editor_sync_failure,
        }
    }

    pub fn buffer_live_op(&self, entry: ConfirmedOp) {
        match self.buffered_live_ops.lock() {
            Ok(mut buffered) => buffered.push(entry),
            Err(_) => leptos::logging::warn!("忽略 live op: buffered_live_ops 锁已损坏"),
        }
    }

    pub fn drain_buffered_live_ops(&self) -> Vec<ConfirmedOp> {
        match self.buffered_live_ops.lock() {
            Ok(mut buffered) => std::mem::take(&mut *buffered),
            Err(_) => {
                leptos::logging::warn!("忽略 buffered live ops: 锁已损坏");
                Vec::new()
            }
        }
    }

    pub fn restore_buffered_live_ops(&self, mut entries: Vec<ConfirmedOp>) {
        match self.buffered_live_ops.lock() {
            Ok(mut buffered) => {
                entries.append(&mut *buffered);
                *buffered = entries;
            }
            Err(_) => leptos::logging::warn!("failed to restore buffered live ops: lock poisoned"),
        }
    }

    pub fn drain_buffered_encrypted_ops(&self) -> Vec<EncryptedOp> {
        match self.buffered_encrypted_ops.lock() {
            Ok(mut buffered) => std::mem::take(&mut *buffered),
            Err(_) => {
                leptos::logging::warn!("忽略 buffered encrypted sync pushes: 锁已损坏");
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EditorSyncFailureSink;
    use crate::runtime::domain::{EditorSyncFailureCode, LoadPhase};
    use leptos::prelude::*;
    use leptos::reactive::owner::Owner;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn editor_sync_failure_sink_sets_error_and_structured_diagnostics() {
        let owner = Owner::new();
        owner.set();
        let (open_request_id, _) = signal(17u64);
        let (load_state, set_load_state) = signal(LoadPhase::Partial);
        let (load_progress, set_load_progress) = signal((2usize, 4usize));
        let (load_eta_ms, set_load_eta_ms) = signal(8u64);
        let (failure, set_failure) = signal(None);
        let ready_generation = Arc::new(AtomicU64::new(9));
        let sink = EditorSyncFailureSink {
            session_generation: Arc::new(AtomicU64::new(11)),
            ready_generation: ready_generation.clone(),
            open_request_id,
            set_load_state,
            set_load_progress,
            set_load_eta_ms,
            set_editor_sync_failure: set_failure,
        };

        sink.fail(EditorSyncFailureCode::ContentReadback);

        assert_eq!(load_state.get_untracked(), LoadPhase::Error);
        assert_eq!(load_progress.get_untracked(), (0, 0));
        assert_eq!(load_eta_ms.get_untracked(), 0);
        assert_eq!(ready_generation.load(Ordering::Relaxed), 0);
        assert_eq!(
            failure.get_untracked(),
            Some(crate::runtime::domain::EditorSyncFailure::new(
                EditorSyncFailureCode::ContentReadback,
                11,
                17,
            ))
        );
    }
}

// apps/web/src/editor/sync/context.rs
//! SyncContext: 将同步参数打包为一个上下文结构体

use crate::api::WsService;
use crate::editor::EditorStats;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::pending::PendingLocalEdits;
use deve_core::models::{DocId, Op, PeerId};
use deve_core::protocol::ConfirmedOp;
use deve_core::security::RepoKey;
use leptos::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 同步消息处理所需的全部上下文
///
/// # Invariants
/// - `doc_id` 在整个编辑器会话中保持不变
/// - `client_id` 唯一标识当前客户端实例
/// - `repo_key` 仅在内存中持有，页面卸载时清除 (NEVER persisted)
pub struct SyncContext<'a> {
    pub doc_id: DocId,
    pub client_id: Option<u64>,
    pub session_generation: Arc<AtomicU64>,
    pub ready_generation: Arc<AtomicU64>,
    pub buffered_live_ops: Arc<Mutex<Vec<ConfirmedOp>>>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub handshake_scope_nonce: ReadSignal<Option<u64>>,
    pub open_request_id: ReadSignal<u64>,
    pub ws: &'a WsService,
    // 内容信号
    pub set_content: WriteSignal<String>,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
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
    pub set_load_state: WriteSignal<String>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub set_load_eta_ms: WriteSignal<u64>,
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
}

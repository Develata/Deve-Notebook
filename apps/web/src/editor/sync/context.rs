// apps/web/src/editor/sync/context.rs
//! SyncContext: 将 14 个同步参数打包为一个上下文结构体

use crate::api::WsService;
use crate::editor::EditorStats;
use deve_core::models::{DocId, Op};
use leptos::prelude::*;

/// 同步消息处理所需的全部上下文 (替代 14 个独立参数)
///
/// # Invariants
/// - `doc_id` 在整个编辑器会话中保持不变
/// - `client_id` 唯一标识当前客户端实例
pub struct SyncContext<'a> {
    pub doc_id: DocId,
    pub client_id: u64,
    pub ws: &'a WsService,
    // 内容信号
    pub set_content: WriteSignal<String>,
    // 版本追踪
    pub local_version: ReadSignal<u64>,
    pub set_local_version: WriteSignal<u64>,
    // 历史记录
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
}

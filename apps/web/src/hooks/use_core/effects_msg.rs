// apps/web/src/hooks/use_core/effects_msg.rs
//! # 消息处理器
//!
//! 处理服务器消息并更新对应信号。

use deve_core::models::{DocId, PeerId};
use deve_core::protocol::ServerMessage;
use leptos::prelude::*;

use super::contexts::SystemMetricsData;
use super::types::{ChatMessage, PeerSession};

/// 处理 DocList 消息。
///
/// 为保持 Dashboard 根页面稳定，不再自动选中首篇文档。
pub fn handle_doc_list(
    list: Vec<(DocId, String)>,
    set_docs: WriteSignal<Vec<(DocId, String)>>,
    _current_doc: ReadSignal<Option<DocId>>,
    _set_current_doc: WriteSignal<Option<DocId>>,
    _explicit_home: ReadSignal<bool>,
) {
    leptos::logging::log!("收到 DocList: {} 篇文档", list.len());
    set_docs.set(list);
}

/// 处理 SyncHello 消息。
pub fn handle_sync_hello(
    peer_id: PeerId,
    vector: deve_core::models::VersionVector,
    set_peers: WriteSignal<std::collections::HashMap<PeerId, PeerSession>>,
) {
    set_peers.update(|map| {
        map.insert(
            peer_id.clone(),
            PeerSession {
                id: peer_id,
                vector,
                last_seen: js_sys::Date::now() as u64,
            },
        );
    });
}

/// 处理 AI 聊天流增量。
pub fn handle_chat_chunk(
    req_id: String,
    delta: Option<String>,
    finish_reason: Option<String>,
    set_chat_messages: WriteSignal<Vec<ChatMessage>>,
    set_is_chat_streaming: WriteSignal<bool>,
) {
    if let Some(delta) = delta.filter(|text| !text.is_empty()) {
        set_chat_messages.update(|messages| {
            if let Some(existing) = messages
                .iter_mut()
                .rev()
                .find(|msg| msg.req_id.as_deref() == Some(req_id.as_str()))
            {
                existing.content.push_str(&delta);
            } else {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: delta,
                    req_id: Some(req_id.clone()),
                    ts_ms: js_sys::Date::now() as u64,
                });
            }
        });
    }

    if finish_reason.is_some() {
        set_is_chat_streaming.set(false);
    }
}

/// 处理分支切换确认。
pub fn handle_branch_switched(
    peer_id: Option<String>,
    success: bool,
    set_active_branch: WriteSignal<Option<PeerId>>,
) {
    if !success {
        leptos::logging::warn!("分支切换失败");
        return;
    }

    set_active_branch.set(peer_id.map(PeerId::new));
}

/// 处理仓库切换确认。
pub fn handle_repo_switched(
    name: String,
    uuid: String,
    set_current_repo: WriteSignal<Option<String>>,
    set_current_repo_id: WriteSignal<Option<String>>,
) {
    set_current_repo.set(Some(name));
    set_current_repo_id.set((!uuid.is_empty()).then_some(uuid));
}

/// 处理剩余的通用消息。
pub fn handle_remaining(
    msg: ServerMessage,
    set_system_metrics: WriteSignal<Option<SystemMetricsData>>,
) {
    match msg {
        ServerMessage::SystemMetrics {
            cpu_usage_percent,
            memory_used_mb,
            active_connections,
            ops_processed,
            uptime_secs,
            db_size_bytes,
            doc_count,
        } => {
            set_system_metrics.set(Some(SystemMetricsData {
                cpu_usage_percent,
                memory_used_mb,
                active_connections,
                ops_processed,
                uptime_secs,
                db_size_bytes,
                doc_count,
            }));
        }
        other => {
            leptos::logging::log!("未处理的服务端消息: {:?}", other);
        }
    }
}

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
    current_doc: ReadSignal<Option<DocId>>,
    set_current_doc: WriteSignal<Option<DocId>>,
    set_docs: WriteSignal<Vec<(DocId, String)>>,
    pending_created_doc_path: ReadSignal<Option<String>>,
    set_pending_created_doc_path: WriteSignal<Option<String>>,
) {
    if let Some(selected) = current_doc.get_untracked()
        && !list.iter().any(|(doc_id, _)| *doc_id == selected)
    {
        leptos::logging::log!("清理过期 current_doc: {} 不在当前 DocList 中", selected);
        set_current_doc.set(None);
    }
    if current_doc.get_untracked().is_none()
        && let Some(pending_path) = pending_created_doc_path.get_untracked()
        && let Some((doc_id, _)) = list.iter().find(|(_, path)| *path == pending_path)
    {
        set_current_doc.set(Some(*doc_id));
        set_pending_created_doc_path.set(None);
    }
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
            }
        });
    }

    if finish_reason.is_some() {
        set_is_chat_streaming.set(false);
    }
}

/// 处理剩余的通用消息。
pub fn handle_remaining(
    msg: ServerMessage,
    set_system_metrics: WriteSignal<Option<SystemMetricsData>>,
) {
    match msg {
        ServerMessage::Pong => {}
        ServerMessage::Snapshot { .. }
        | ServerMessage::History { .. }
        | ServerMessage::NewOp { .. }
        | ServerMessage::SyncPush { .. }
        | ServerMessage::SyncPushSnapshot { .. }
        | ServerMessage::KeyProvide { .. }
        | ServerMessage::KeyDenied { .. } => {}
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

#[cfg(test)]
mod tests {
    use super::ChatMessage;
    use super::{handle_chat_chunk, handle_doc_list};
    use deve_core::models::DocId;
    use leptos::prelude::*;

    #[test]
    fn chat_chunk_ignores_unknown_req_after_scope_reset() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
        let (_streaming, set_streaming) = signal(true);

        handle_chat_chunk(
            "req-1".into(),
            Some("late".into()),
            None,
            set_messages,
            set_streaming,
        );

        assert!(messages.get_untracked().is_empty());
    }

    #[test]
    fn doc_list_clears_stale_current_doc() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let stale = DocId::new();
        let fresh = DocId::new();
        let (current_doc, set_current_doc) = signal(Some(stale));
        let (_docs, set_docs) = signal(Vec::<(DocId, String)>::new());
        let (pending_created_doc_path, set_pending_created_doc_path) = signal(None::<String>);

        handle_doc_list(
            vec![(fresh, "notes/fresh.md".into())],
            current_doc,
            set_current_doc,
            set_docs,
            pending_created_doc_path,
            set_pending_created_doc_path,
        );

        assert_eq!(current_doc.get_untracked(), None);
    }

    #[test]
    fn doc_list_preserves_current_doc_when_it_still_exists() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let selected = DocId::new();
        let (current_doc, set_current_doc) = signal(Some(selected));
        let (_docs, set_docs) = signal(Vec::<(DocId, String)>::new());
        let (pending_created_doc_path, set_pending_created_doc_path) = signal(None::<String>);

        handle_doc_list(
            vec![(selected, "notes/selected.md".into())],
            current_doc,
            set_current_doc,
            set_docs,
            pending_created_doc_path,
            set_pending_created_doc_path,
        );

        assert_eq!(current_doc.get_untracked(), Some(selected));
    }

    #[test]
    fn doc_list_selects_pending_created_doc_when_no_doc_is_open() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let created = DocId::new();
        let (current_doc, set_current_doc) = signal(None::<DocId>);
        let (_docs, set_docs) = signal(Vec::<(DocId, String)>::new());
        let (pending_created_doc_path, set_pending_created_doc_path) =
            signal(Some("Untitled.md".to_string()));

        handle_doc_list(
            vec![(created, "Untitled.md".into())],
            current_doc,
            set_current_doc,
            set_docs,
            pending_created_doc_path,
            set_pending_created_doc_path,
        );

        assert_eq!(current_doc.get_untracked(), Some(created));
        assert_eq!(pending_created_doc_path.get_untracked(), None);
    }
}

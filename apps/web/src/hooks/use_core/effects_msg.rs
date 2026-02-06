// apps/web/src/hooks/use_core/effects_msg.rs
//! # 消息处理器
//!
//! 处理服务器消息并更新对应信号。

use crate::api::WsService;
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::types::{ChatMessage, PeerSession};

/// 处理 DocList 消息
pub fn handle_doc_list(
    list: Vec<(DocId, String)>,
    set_docs: WriteSignal<Vec<(DocId, String)>>,
    current_doc: ReadSignal<Option<DocId>>,
    set_current_doc: WriteSignal<Option<DocId>>,
) {
    leptos::logging::log!("收到 DocList: {} 篇文档", list.len());
    set_docs.set(list.clone());
    if current_doc.get_untracked().is_none()
        && let Some(first) = list.first()
    {
        set_current_doc.set(Some(first.0));
    }
}

/// 处理 SyncHello 消息
pub fn handle_sync_hello(
    peer_id: PeerId,
    vector: deve_core::models::VersionVector,
    set_peers: WriteSignal<std::collections::HashMap<PeerId, PeerSession>>,
) {
    set_peers.update(|map| {
        map.insert(
            peer_id.clone(),
            PeerSession {
                id: peer_id.clone(),
                vector,
                last_seen: js_sys::Date::now() as u64,
            },
        );
    });
}

/// 处理 ChatChunk 消息 (流式 AI 响应)
pub fn handle_chat_chunk(
    req_id: String,
    delta: Option<String>,
    finish_reason: Option<String>,
    set_chat_messages: WriteSignal<Vec<ChatMessage>>,
    set_is_chat_streaming: WriteSignal<bool>,
) {
    if let Some(text) = delta {
        set_chat_messages.update(|msgs| {
            if let Some(msg) = msgs
                .iter_mut()
                .rev()
                .find(|m| m.req_id.as_deref() == Some(&req_id))
            {
                msg.content.push_str(&text);
            } else {
                msgs.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: text,
                    req_id: Some(req_id.clone()),
                    ts_ms: js_sys::Date::now() as u64,
                });
            }
        });
        set_is_chat_streaming.set(true);
    }
    if finish_reason.is_some() {
        set_is_chat_streaming.set(false);
    }
}

/// 处理 BranchSwitched 消息
pub fn handle_branch_switched(
    ws: &WsService,
    peer_id: Option<String>,
    success: bool,
    current_doc: ReadSignal<Option<DocId>>,
    set_active_branch: WriteSignal<Option<PeerId>>,
) {
    leptos::logging::log!("分支已切换到 {:?}, 成功: {}", peer_id, success);
    if success {
        let new_val = peer_id.clone().map(PeerId::new);
        set_active_branch.set(new_val);

        if let Some(doc_id) = current_doc.get_untracked() {
            ws.send(ClientMessage::OpenDoc { doc_id });
        }
    }
}

/// 处理 RepoSwitched 消息
pub fn handle_repo_switched(
    ws: &WsService,
    name: String,
    current_doc: ReadSignal<Option<DocId>>,
    set_current_repo: WriteSignal<Option<String>>,
) {
    leptos::logging::log!("仓库已切换到: {}", name);
    set_current_repo.set(Some(name));

    if let Some(doc_id) = current_doc.get_untracked() {
        ws.send(ClientMessage::OpenDoc { doc_id });
    }

    ws.send(ClientMessage::GetChanges);
    ws.send(ClientMessage::GetCommitHistory { limit: 50 });
}

// apps/web/src/hooks/use_core/effects_msg.rs
//! # 消息处理器
//!
//! 处理服务器消息并更新对应信号。

use deve_core::models::{DocId, PeerId};
use deve_core::protocol::ServerMessage;
use leptos::prelude::*;

use super::contexts::SystemMetricsData;
use super::types::{ChatMessage, PeerSession, PendingBranchTarget};

/// 处理 DocList 消息。
///
/// 为保持 Dashboard 根页面稳定，不再自动选中首篇文档。
pub fn handle_doc_list(list: Vec<(DocId, String)>, set_docs: WriteSignal<Vec<(DocId, String)>>) {
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
    active_branch: ReadSignal<Option<PeerId>>,
    pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    set_pending_branch_switch: WriteSignal<Option<PendingBranchTarget>>,
    set_active_branch: WriteSignal<Option<PeerId>>,
) -> bool {
    let next_target = peer_id
        .clone()
        .map(PendingBranchTarget::Shadow)
        .unwrap_or(PendingBranchTarget::Local);
    if let Some(pending) = pending_branch_switch.get_untracked() {
        if pending != next_target {
            leptos::logging::warn!("忽略过期 BranchSwitched: {:?}", peer_id);
            return false;
        }
        set_pending_branch_switch.set(None);
    }
    if !success {
        leptos::logging::warn!("分支切换失败");
        return false;
    }

    let next_branch = peer_id.map(PeerId::new);
    let changed = active_branch.get_untracked() != next_branch;
    set_active_branch.set(next_branch);
    changed
}

/// 处理仓库切换确认。
pub fn handle_repo_switched(
    name: String,
    uuid: String,
    current_repo_id: ReadSignal<Option<String>>,
    pending_repo_switch: ReadSignal<Option<String>>,
    set_pending_repo_switch: WriteSignal<Option<String>>,
    set_current_repo: WriteSignal<Option<String>>,
    set_current_repo_id: WriteSignal<Option<String>>,
    set_current_doc: WriteSignal<Option<DocId>>,
) -> bool {
    if let Some(pending) = pending_repo_switch.get_untracked() {
        if pending != name {
            leptos::logging::warn!("忽略过期 RepoSwitched: {}", name);
            return false;
        }
        set_pending_repo_switch.set(None);
    }
    let same_repo =
        !uuid.is_empty() && current_repo_id.get_untracked().as_deref() == Some(uuid.as_str());
    set_current_repo.set(Some(name));
    set_current_repo_id.set((!uuid.is_empty()).then_some(uuid));
    if !same_repo {
        set_current_doc.set(None);
    }
    !same_repo
}

#[cfg(test)]
mod tests {
    use super::handle_branch_switched;
    use super::handle_repo_switched;
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::models::DocId;
    use deve_core::models::PeerId;
    use leptos::prelude::*;
    use uuid::Uuid;

    #[test]
    fn clears_doc_when_repo_uuid_changes_even_if_name_matches() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();

        let (_, set_current_repo) = signal(Some("default".to_string()));
        let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
        let (pending_repo_switch, set_pending_repo_switch) = signal(Some("default".to_string()));
        let (current_doc, set_current_doc) = signal(Some(DocId::new()));
        let next_repo_id = Uuid::new_v4().to_string();

        let changed = handle_repo_switched(
            "default".to_string(),
            next_repo_id.clone(),
            current_repo_id,
            pending_repo_switch,
            set_pending_repo_switch,
            set_current_repo,
            set_current_repo_id,
            set_current_doc,
        );

        assert!(changed);
        assert_eq!(current_repo_id.get_untracked(), Some(next_repo_id));
        assert_eq!(current_doc.get_untracked(), None);
        assert_eq!(pending_repo_switch.get_untracked(), None);
    }

    #[test]
    fn branch_switch_reports_when_scope_changed() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();

        let (active_branch, set_active_branch) = signal(Some(PeerId::new("peer-a")));
        let (pending_branch_switch, set_pending_branch_switch) =
            signal(Some(PendingBranchTarget::Shadow("peer-b".into())));
        let changed = handle_branch_switched(
            Some("peer-b".into()),
            true,
            active_branch,
            pending_branch_switch,
            set_pending_branch_switch,
            set_active_branch,
        );

        assert!(changed);
        assert_eq!(active_branch.get_untracked(), Some(PeerId::new("peer-b")));
        assert_eq!(pending_branch_switch.get_untracked(), None);
    }

    #[test]
    fn ignores_stale_repo_switched_while_newer_target_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();

        let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
        let (_, set_current_repo) = signal(Some("test".to_string()));
        let (pending_repo_switch, set_pending_repo_switch) = signal(Some("default".to_string()));
        let (_current_doc, set_current_doc) = signal(Some(DocId::new()));

        let changed = handle_repo_switched(
            "stale".to_string(),
            Uuid::new_v4().to_string(),
            current_repo_id,
            pending_repo_switch,
            set_pending_repo_switch,
            set_current_repo,
            set_current_repo_id,
            set_current_doc,
        );

        assert!(!changed);
        assert_eq!(
            pending_repo_switch.get_untracked(),
            Some("default".to_string())
        );
    }
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

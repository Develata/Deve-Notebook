//! 文件创建与已有文件注册逻辑。

use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::broadcast_parent_dirs;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::repo_scope::{ResolvedRepo, run_on_resolved_local_repo};
use crate::server::session::WsSession;
use anyhow::anyhow;
use deve_core::ledger::node_meta;
use deve_core::models::{DocId, NodeId};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub async fn handle_file_create(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    path: &std::path::Path,
    filename: &str,
) {
    let doc_id = if path.exists() {
        match register_existing_file(state, scope, filename) {
            Some(doc_id) => doc_id,
            None => return,
        }
    } else if let Err(e) = std::fs::write(path, "") {
        tracing::error!("创建文件失败: {:?}", e);
        ch.send_error(format!("Failed to create file: {}", e));
        return;
    } else {
        match state
            .repo
            .create_docid_in_local_repo(&scope.repo_name, filename)
        {
            Ok(doc_id) => {
                tracing::info!("已创建文档: {} ({})", filename, doc_id);
                doc_id
            }
            Err(e) => {
                tracing::error!("Ledger 注册失败: {:?}", e);
                return;
            }
        }
    };

    push_file_tree_update(state, ch, scope, doc_id);
    handle_list_docs(state, ch, session).await;
    notify_fs_refresh(ch, scope.repo_id, filename, "added");
}

fn register_existing_file(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    filename: &str,
) -> Option<DocId> {
    match state
        .repo
        .get_docid_in_local_repo(&scope.repo_name, filename)
    {
        Ok(Some(doc_id)) => Some(doc_id),
        Ok(None) => match state
            .repo
            .create_docid_in_local_repo(&scope.repo_name, filename)
        {
            Ok(doc_id) => Some(doc_id),
            Err(e) => {
                tracing::error!("Ledger 注册失败: {:?}", e);
                None
            }
        },
        Err(e) => {
            tracing::error!("DocId 获取失败: {:?}", e);
            None
        }
    }
}

fn push_file_tree_update(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    doc_id: DocId,
) {
    let node_id = NodeId::from_doc_id(doc_id);
    if let Ok(meta) = run_on_resolved_local_repo(state, scope, |db| {
        node_meta::get_node_meta(db, node_id)
            .and_then(|meta| meta.ok_or_else(|| anyhow!("File node meta missing")))
    }) {
        if let Err(e) =
            broadcast_parent_dirs(state, ch, scope.repo_id, &scope.repo_name, meta.parent_id)
        {
            tracing::error!("广播父目录失败: {:?}", e);
        }
        let delta = state.tree_manager.with_tree_mut(scope.repo_id, |tm| {
            tm.add_file(
                node_id,
                meta.path.clone(),
                meta.parent_id,
                meta.name.clone(),
                doc_id,
            )
        });
        ch.unicast(ServerMessage::TreeUpdate(delta));
    }
}

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
use deve_core::state;
use deve_core::sync::reconcile;
use std::sync::Arc;

pub async fn handle_file_create(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    path: &std::path::Path,
    filename: &str,
) {
    if path.exists() && !path.is_file() {
        tracing::error!("目标路径不是文件: {:?}", path);
        ch.send_error("Target path is not a file".to_string());
        return;
    }

    let existing_doc_id = match state
        .repo
        .get_docid_in_local_repo(&scope.repo_name, filename)
    {
        Ok(doc_id) => doc_id,
        Err(e) => {
            tracing::error!("DocId 获取失败: {:?}", e);
            ch.send_error(format!("Failed to resolve doc id: {}", e));
            return;
        }
    };
    let disk_content = if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(content) => Some(content),
            Err(e) => {
                tracing::error!("读取现有文件失败: {:?}", e);
                ch.send_error(format!("Failed to read existing file: {}", e));
                return;
            }
        }
    } else {
        None
    };
    let doc_id = match state.repo.apply_file_structure_in_local_repo(
        &scope.repo_name,
        filename,
        existing_doc_id,
        "local_create",
    ) {
        Ok(doc_id) => doc_id,
        Err(e) => {
            tracing::error!("Structure Facts 追加失败: {:?}", e);
            ch.send_error(format!("Failed to register file: {}", e));
            return;
        }
    };

    if let Some(content) = disk_content {
        if existing_doc_id.is_none()
            && !content.is_empty()
            && let Err(e) = reconcile::append_patch_in_local_repo(
                state.repo.as_ref(),
                &scope.repo_name,
                doc_id,
                "local_create",
                &state::compute_diff("", &content),
            )
        {
            tracing::error!("导入现有文件内容失败: {:?}", e);
            ch.send_error(format!("Failed to import existing file: {}", e));
            return;
        }
    } else if let Err(e) = state
        .sync_manager
        .persist_doc_in_local_repo(&scope.repo_name, doc_id)
    {
        tracing::error!("投影创建文件失败: {:?}", e);
        ch.send_error(format!("Failed to materialize file: {}", e));
        return;
    };

    push_file_tree_update(state, ch, scope, doc_id);
    handle_list_docs(state, ch, session).await;
    notify_fs_refresh(ch, scope.repo_id, filename, "added");
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

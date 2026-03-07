//! 目录创建逻辑。

use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::broadcast_dir_chain;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::repo_scope::{ResolvedRepo, run_on_resolved_local_repo};
use crate::server::session::WsSession;
use anyhow::anyhow;
use deve_core::ledger::node_meta;
use std::sync::Arc;

pub async fn handle_folder_create(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    path: &std::path::Path,
    filename: &str,
) {
    if path.exists() {
        if !path.is_dir() {
            tracing::error!("目标路径不是目录: {:?}", path);
            ch.send_error("Target path is not a directory".to_string());
            return;
        }
        tracing::debug!("文件夹已存在: {:?}", path);
    } else if let Err(e) = std::fs::create_dir_all(path) {
        tracing::error!("创建文件夹失败: {:?}", e);
        ch.send_error(format!("Failed to create folder: {}", e));
        return;
    } else {
        tracing::info!("已创建文件夹: {}", filename);
    }

    let folder_path = filename.trim_end_matches('/');
    let created = run_on_resolved_local_repo(state, scope, |db| {
        if node_meta::get_node_id(db, folder_path)?.is_some() {
            return Ok(None);
        }
        let node_id = node_meta::create_dir_node(db, folder_path)?;
        let meta = node_meta::get_node_meta(db, node_id)?
            .ok_or_else(|| anyhow!("Dir node meta missing: {}", folder_path))?;
        Ok(Some((node_id, meta)))
    });

    if let Ok(Some((node_id, _meta))) = created
        && let Err(e) = broadcast_dir_chain(state, ch, scope.repo_id, &scope.repo_name, node_id)
    {
        tracing::error!("广播目录链失败: {:?}", e);
    }
    handle_list_docs(state, ch, session).await;
    notify_fs_refresh(ch, scope.repo_id, folder_path, "dir-added");
}

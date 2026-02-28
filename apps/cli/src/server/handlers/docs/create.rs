// apps/cli/src/server/handlers/docs/create.rs
//! # 创建文档处理器

use super::validate_path;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::{broadcast_dir_chain, broadcast_parent_dirs};
use crate::server::handlers::listing::handle_list_docs;
use crate::server::session::WsSession;
use anyhow::anyhow;
use deve_core::ledger::node_meta;
use deve_core::models::NodeId;
use deve_core::protocol::ServerMessage;
use deve_core::utils::path::join_normalized;
use std::sync::Arc;

/// 处理创建文档请求
///
/// **流程**:
/// 1. 校验文件名 (防止遍历攻击、深度超限)
/// 2. 确保父目录存在
/// 3. 创建文件并写入默认内容
/// 4. 在 Ledger 中注册 DocId
/// 5. 更新 TreeManager 并广播 TreeDelta
pub async fn handle_create_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    name: String,
) {
    // 只读模式检查: 静默忽略创建请求
    // TODO: Frontend will hide create buttons when readonly
    if session.is_readonly() {
        tracing::debug!("Create ignored: session is readonly (remote branch)");
        return;
    }

    // 1. 处理文件名: 文件夹路径(以/结尾)保持原样，普通文件确保以 .md 结尾
    let mut filename = name.clone();
    if !filename.ends_with('/') && !filename.ends_with(".md") {
        filename.push_str(".md");
    }

    // 2. 路径校验
    if !validate_path(&filename, ch) {
        return;
    }

    // 3. 构建完整路径
    let path = join_normalized(&state.vault_path, &filename);

    // 4. 确保父目录存在
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::error!("创建目录失败: {:?}", e);
        ch.send_error(format!("Failed to create directories: {}", e));
        return;
    }

    // 5. 根据路径类型创建文件或文件夹
    let is_folder = filename.ends_with('/');

    if is_folder {
        // 创建文件夹
        if path.exists() {
            if !path.is_dir() {
                tracing::error!("目标路径不是目录: {:?}", path);
                ch.send_error("Target path is not a directory".to_string());
                return;
            }
            tracing::debug!("文件夹已存在: {:?}", path);
        } else if let Err(e) = std::fs::create_dir_all(&path) {
            tracing::error!("创建文件夹失败: {:?}", e);
            ch.send_error(format!("Failed to create folder: {}", e));
            return;
        } else {
            tracing::info!("已创建文件夹: {}", filename);
        }
        // 更新 Node 表与 TreeManager
        let folder_path = filename.trim_end_matches('/');
        let created = state
            .repo
            .run_on_local_repo(state.repo.local_repo_name(), |db| {
                if node_meta::get_node_id(db, folder_path)?.is_some() {
                    return Ok(None);
                }
                let node_id = node_meta::create_dir_node(db, folder_path)?;
                let meta = node_meta::get_node_meta(db, node_id)?
                    .ok_or_else(|| anyhow!("Dir node meta missing: {}", folder_path))?;
                Ok(Some((node_id, meta)))
            });

        if let Ok(Some((node_id, _meta))) = created
            && let Err(e) = broadcast_dir_chain(state, ch, node_id)
        {
            tracing::error!("广播目录链失败: {:?}", e);
        }
    } else if path.exists() {
        // 文件已存在，仅注册到 Ledger
        let doc_id = match state.repo.get_docid(&filename) {
            Ok(Some(id)) => id,
            Ok(None) => match state.repo.create_docid(&filename) {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!("Ledger 注册失败: {:?}", e);
                    return;
                }
            },
            Err(e) => {
                tracing::error!("DocId 获取失败: {:?}", e);
                return;
            }
        };

        let node_id = NodeId::from_doc_id(doc_id);
        if let Ok(meta) = state
            .repo
            .run_on_local_repo(state.repo.local_repo_name(), |db| {
                node_meta::get_node_meta(db, node_id)
                    .and_then(|m| m.ok_or_else(|| anyhow!("File node meta missing")))
            })
        {
            if let Err(e) = broadcast_parent_dirs(state, ch, meta.parent_id) {
                tracing::error!("广播父目录失败: {:?}", e);
            }
            let delta = state.tree_manager.write().unwrap_or_else(|e| e.into_inner()).add_file(
                node_id,
                meta.path.clone(),
                meta.parent_id,
                meta.name.clone(),
                doc_id,
            );
            ch.broadcast(ServerMessage::TreeUpdate(delta));
        }
        // 刷新文档列表
        handle_list_docs(state, ch, session).await;
    } else if let Err(e) = std::fs::write(&path, "") {
        tracing::error!("创建文件失败: {:?}", e);
        ch.send_error(format!("Failed to create file: {}", e));
    } else if let Ok(doc_id) = state.repo.create_docid(&filename) {
        tracing::info!("已创建文档: {} ({})", filename, doc_id);
        let node_id = NodeId::from_doc_id(doc_id);
        if let Ok(meta) = state
            .repo
            .run_on_local_repo(state.repo.local_repo_name(), |db| {
                node_meta::get_node_meta(db, node_id)
                    .and_then(|m| m.ok_or_else(|| anyhow!("File node meta missing")))
            })
        {
            if let Err(e) = broadcast_parent_dirs(state, ch, meta.parent_id) {
                tracing::error!("广播父目录失败: {:?}", e);
            }
            let delta = state.tree_manager.write().unwrap().add_file(
                node_id,
                meta.path.clone(),
                meta.parent_id,
                meta.name.clone(),
                doc_id,
            );
            ch.broadcast(ServerMessage::TreeUpdate(delta));
        }
        // 刷新文档列表
        handle_list_docs(state, ch, session).await;
    }
}

// apps/cli/src/server/handlers/docs.rs
//! # 文档 CRUD 处理器
//!
//! 处理文档的创建、重命名、删除、复制、移动操作

use super::listing::handle_list_docs;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::protocol::ServerMessage;
use deve_core::utils::path::join_normalized;
use std::sync::Arc;

/// 处理创建文档请求
pub async fn handle_create_doc(state: &Arc<AppState>, ch: &DualChannel, name: String) {
    let mut filename = name.clone();
    if !filename.ends_with(".md") {
        filename.push_str(".md");
    }

    // 防止目录遍历攻击
    if filename.contains("..") || filename.starts_with('/') || filename.starts_with('\\') {
        tracing::error!("Invalid filename: {}", filename);
        ch.send_error(format!("Invalid filename: {}", filename));
        return;
    }

    let path = join_normalized(&state.vault_path, &filename);

    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::error!("Failed to create directories: {:?}", e);
            ch.send_error(format!("Failed to create directories: {}", e));
            return;
        }
    }

    if path.exists() {
        if let Ok(_doc_id) = state.repo.create_docid(&filename) {
            handle_list_docs(state, ch, None).await;
        }
    } else if let Err(e) = std::fs::write(&path, "# New Note\n") {
        tracing::error!("Failed to create file: {:?}", e);
        ch.send_error(format!("Failed to create file: {}", e));
    } else if let Ok(doc_id) = state.repo.create_docid(&filename) {
        tracing::info!("Created doc: {} ({})", filename, doc_id);
        handle_list_docs(state, ch, None).await;
    }
}

/// 处理重命名文档请求
pub async fn handle_rename_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    old_path: String,
    new_path: String,
) {
    let src = join_normalized(&state.vault_path, &old_path);

    let mut dst_name = new_path.clone();
    if !dst_name.ends_with(".md") && old_path.ends_with(".md") {
        dst_name.push_str(".md");
    }

    let dst = join_normalized(&state.vault_path, &dst_name);

    if src.exists() {
        if let Some(parent) = dst.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Err(e) = std::fs::rename(&src, &dst) {
            tracing::error!("Failed to rename {} to {}: {:?}", old_path, dst_name, e);
            ch.send_error(format!("Failed to rename: {}", e));
        } else {
            tracing::info!("Renamed {} to {}", old_path, dst_name);

            if dst.is_dir() {
                if let Err(e) = state.repo.rename_folder(&old_path, &dst_name) {
                    tracing::error!("Failed to update ledger rename folder: {:?}", e);
                }
            } else if let Err(e) = state.repo.rename_doc(&old_path, &dst_name) {
                tracing::error!("Failed to update ledger rename doc: {:?}", e);
            }

            handle_list_docs(state, ch, None).await;
        }
    } else {
        tracing::warn!("Rename failed: Source does not exist: {:?}", src);
        ch.send_error(format!("Source not found: {}", old_path));
    }
}

/// 处理删除文档请求
pub async fn handle_delete_doc(state: &Arc<AppState>, ch: &DualChannel, path: String) {
    tracing::info!("handle_delete_doc called with path={}", path);
    let target = join_normalized(&state.vault_path, &path);
    let is_dir = target.is_dir();

    if target.exists() {
        if is_dir {
            if let Err(e) = std::fs::remove_dir_all(&target) {
                tracing::error!("Failed to delete dir {}: {:?}", path, e);
            }
        } else if let Err(e) = std::fs::remove_file(&target) {
            tracing::error!("Failed to delete file {}: {:?}", path, e);
        }
    } else {
        tracing::warn!("File to delete not found: {:?}", target);
    }

    if is_dir {
        match state.repo.delete_folder(&path) {
            Ok(count) => tracing::info!("Deleted {} docs from ledger for folder {}", count, path),
            Err(e) => tracing::error!("Failed to delete folder from ledger: {:?}", e),
        }
    } else if let Err(e) = state.repo.delete_doc(&path) {
        tracing::error!("Failed to delete doc from ledger: {:?}", e);
    }

    handle_list_docs(state, ch, None).await;
}

/// 处理复制文档请求
pub async fn handle_copy_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    src_path: String,
    dest_path: String,
) {
    let src = join_normalized(&state.vault_path, &src_path);
    let dst = join_normalized(&state.vault_path, &dest_path);

    if !src.exists() {
        tracing::error!("Copy failed: Source not found: {:?}", src);
        ch.send_error(format!("Source not found: {}", src_path));
        return;
    }

    if dst.exists() {
        tracing::error!("Copy failed: Destination already exists: {:?}", dst);
        ch.send_error(format!("Destination exists: {}", dest_path));
        return;
    }

    if let Some(parent) = dst.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if src.is_dir() {
        tracing::warn!("Copying directory is not fully supported yet in MVP");
        ch.send_error("Directory copy not supported".to_string());
    } else if let Err(e) = std::fs::copy(&src, &dst) {
        tracing::error!("Failed to copy {} to {:?}: {:?}", src_path, dst, e);
        ch.send_error(format!("Copy failed: {}", e));
    } else if let Ok(doc_id) = state.repo.create_docid(&dest_path) {
        tracing::info!(
            "Copied {} to {} (New DocId: {})",
            src_path,
            dest_path,
            doc_id
        );
        handle_list_docs(state, ch, None).await;
    } else {
        tracing::error!("Failed to register copied doc in ledger");
    }
}

/// 处理移动文档请求
pub async fn handle_move_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    src_path: String,
    dest_path: String,
) {
    handle_rename_doc(state, ch, src_path, dest_path).await;
}

//! # System Handlers (系统操作处理器)
//!
//! **架构作用**:
//! 处理与文件系统和 Ledger 相关的系统级操作请求。
//!
//! **核心功能清单**:
//! - `handle_list_docs`: 列出 Vault 中的所有文档。
//! - `handle_create_doc`: 创建新文档或文件夹。
//! - `handle_rename_doc`: 重命名文档或文件夹，并更新 Ledger。
//! - `handle_delete_doc`: 删除文档或文件夹，并更新 Ledger。
//! - `handle_copy_doc`: 复制文档，并注册新 DocId。
//! - `handle_move_doc`: 移动文档（本质上是重命名）。
//!
//! **类型**: Core MUST (核心必选)

use std::sync::Arc;
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use deve_core::utils::path::join_normalized;
use crate::server::AppState;

use deve_core::models::PeerId;
use deve_core::ledger::RepoType;

pub async fn handle_list_docs(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    active_branch: Option<&PeerId>,
) {
     let repo_type = match active_branch {
         Some(peer_id) => RepoType::Remote(peer_id.clone(), uuid::Uuid::nil()), // Default RepoId
         None => RepoType::Local(uuid::Uuid::nil()),
     };

     if let Ok(docs) = state.repo.list_docs(&repo_type) {
         let msg = ServerMessage::DocList { docs };
         let _ = tx.send(msg);
     }
}

pub async fn handle_create_doc(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    name: String,
) {
    let mut filename = name.clone();
    if !filename.ends_with(".md") {
        filename.push_str(".md");
    }
    
    // 防止目录遍历攻击 (简单的安全检查)
    // 允许使用正斜杠来创建子文件夹 (如 "folder/file.md")
    if filename.contains("..") || filename.starts_with('/') || filename.starts_with('\\') {
         tracing::error!("Invalid filename: {}", filename);
         return; 
    }
        
    // 使用规范化路径以保证跨平台兼容性
    let path = join_normalized(&state.vault_path, &filename);
    
    // 确保父目录存在
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::error!("Failed to create directories: {:?}", e);
            return;
        }
    }

    if path.exists() {
         // 文件已存在？仅注册 ID
         if let Ok(_doc_id) = state.repo.create_docid(&filename) {
              // 广播更新列表
              handle_list_docs(state, tx, None).await;
         }
    } else {
         // 新文件：写入标题或空内容
         if let Err(e) = std::fs::write(&path, "# New Note\n") {
             tracing::error!("Failed to create file: {:?}", e);
         } else {
             if let Ok(doc_id) = state.repo.create_docid(&filename) {
                 // 成功
                 tracing::info!("Created doc: {} ({})", filename, doc_id);
                 
                 // 广播更新列表
                 handle_list_docs(state, tx, None).await;
             }
         }
    }
}

pub async fn handle_rename_doc(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
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
         } else {
              tracing::info!("Renamed {} to {}", old_path, dst_name);
              
              // Update Ledger
              if dst.is_dir() {
                  if let Err(e) = state.repo.rename_folder(&old_path, &dst_name) {
                      tracing::error!("Failed to update ledger rename folder: {:?}", e);
                  }
              } else {
                  if let Err(e) = state.repo.rename_doc(&old_path, &dst_name) {
                      tracing::error!("Failed to update ledger rename doc: {:?}", e);
                  }
              }

              handle_list_docs(state, tx, None).await;
         }
     } else {
         tracing::warn!("Rename failed: Source does not exist: {:?}", src);
     }
}

pub async fn handle_delete_doc(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    path: String,
) {
    tracing::info!("handle_delete_doc called with path={}", path);
    let target = join_normalized(&state.vault_path, &path);
    tracing::info!("target path resolved to: {:?}, exists={}, is_dir={}", target, target.exists(), target.is_dir());
    let is_dir = target.is_dir();
    
    if target.exists() {
        // Check if directory
         if is_dir {
             if let Err(e) = std::fs::remove_dir_all(&target) {
                 tracing::error!("Failed to delete dir {}: {:?}", path, e);
             } else {
                 tracing::info!("Deleted dir {}", path);
             }
         } else {
             if let Err(e) = std::fs::remove_file(&target) {
                 tracing::error!("Failed to delete file {}: {:?}", path, e);
             } else {
                 tracing::info!("Deleted file {}", path);
             }
         }
    } else {
        tracing::warn!("File to delete not found: {:?}, removing from ledger anyway.", target);
    }
         
    // Update Ledger - use appropriate method based on whether it's a folder
    if is_dir {
        // Delete all documents under this folder prefix
        match state.repo.delete_folder(&path) {
            Ok(count) => tracing::info!("Deleted {} docs from ledger for folder {}", count, path),
            Err(e) => tracing::error!("Failed to update ledger delete_folder: {:?}", e),
        }
    } else {
        // Delete single document
        if let Err(e) = state.repo.delete_doc(&path) {
            tracing::error!("Failed to update ledger delete: {:?}", e);
        }
    }
    
    // Update List
    handle_list_docs(state, tx, None).await;
}

pub async fn handle_copy_doc(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    src_path: String,
    dest_path: String,
) {
    let src = join_normalized(&state.vault_path, &src_path);
    let dst = join_normalized(&state.vault_path, &dest_path);

    if !src.exists() {
        tracing::error!("Copy failed: Source not found: {:?}", src);
        return;
    }

    if dst.exists() {
        tracing::error!("Copy failed: Destination already exists: {:?}", dst);
        return;
    }

    if let Some(parent) = dst.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if src.is_dir() {
        // Recursive copy not fully implemented for MVP, but basic folder structure creation
        tracing::warn!("Copying directory is not fully supported yet in MVP (recursive copy needed)");
        // TODO: Implement recursive copy
        return;
    } else {
        if let Err(e) = std::fs::copy(&src, &dst) {
             tracing::error!("Failed to copy {} to {:?}: {:?}", src_path, dst, e);
             return;
        }

        // Register new document in Ledger
        if let Ok(doc_id) = state.repo.create_docid(&dest_path) {
            tracing::info!("Copied {} to {} (New DocId: {})", src_path, dest_path, doc_id);
            handle_list_docs(state, tx, None).await;
        } else {
             tracing::error!("Failed to register copied doc in ledger");
        }
    }
}

pub async fn handle_move_doc(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    src_path: String,
    dest_path: String,
) {
    // Reuse rename logic as it is essentially a move
    handle_rename_doc(state, tx, src_path, dest_path).await;
}

/// 处理 ListShadows 请求 - 返回影子库列表 (远程分支)
pub async fn handle_list_shadows(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
    tracing::info!("Handling ListShadows request. Remotes dir: {:?}", state.repo.remotes_dir());
    match state.repo.list_shadows_on_disk() {
        Ok(peers) => {
            let shadows: Vec<String> = peers.iter()
                .map(|p| p.to_string())
                .collect();
            tracing::info!("Found {} shadow repos: {:?}", shadows.len(), shadows);
            let _ = tx.send(ServerMessage::ShadowList { shadows });
        }
        Err(e) => {
            tracing::error!("Failed to list shadow repos: {:?}", e);
            let _ = tx.send(ServerMessage::ShadowList { shadows: vec![] });
        }
    }
}


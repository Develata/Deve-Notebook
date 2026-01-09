use std::sync::Arc;
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use crate::server::AppState;

pub async fn handle_list_docs(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
     if let Ok(docs) = state.ledger.list_docs() {
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
    
    // Prevent directory traversal (basic check)
    if filename.contains("..") || filename.starts_with('/') || filename.contains('\\') {
         tracing::error!("Invalid filename: {}", filename);
         return; 
    }
        
    let path = state.vault_path.join(&filename);
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::error!("Failed to create directories: {:?}", e);
            return;
        }
    }

    if path.exists() {
         // Already exists? Just register/get ID
         if let Ok(_doc_id) = state.ledger.create_docid(&filename) {
              // Send updated list
              handle_list_docs(state, tx).await;
         }
    } else {
         // New file: Write headers or empty
         if let Err(e) = std::fs::write(&path, "# New Note\n") {
             tracing::error!("Failed to create file: {:?}", e);
         } else {
             if let Ok(doc_id) = state.ledger.create_docid(&filename) {
                 // Success
                 tracing::info!("Created doc: {} ({})", filename, doc_id);
                 
                 // Broadcast List Update
                 handle_list_docs(state, tx).await;
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
     let src = state.vault_path.join(&old_path);
     let mut dst_name = new_path.clone();
     if !dst_name.ends_with(".md") && old_path.ends_with(".md") {
         dst_name.push_str(".md");
     }
     
     let dst = state.vault_path.join(&dst_name);
     
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
                  if let Err(e) = state.ledger.rename_folder(&old_path, &dst_name) {
                      tracing::error!("Failed to update ledger rename folder: {:?}", e);
                  }
              } else {
                  if let Err(e) = state.ledger.rename_doc(&old_path, &dst_name) {
                      tracing::error!("Failed to update ledger rename doc: {:?}", e);
                  }
              }

              handle_list_docs(state, tx).await;
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
    let target = state.vault_path.join(&path);
    if target.exists() {
        // Check if directory
         if target.is_dir() {
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
         
    // Update Ledger ALWAYS
    if let Err(e) = state.ledger.delete_doc(&path) {
        tracing::error!("Failed to update ledger delete: {:?}", e);
    }
    
    // Update List
    handle_list_docs(state, tx).await;
}

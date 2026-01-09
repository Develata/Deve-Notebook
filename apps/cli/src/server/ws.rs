use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use std::sync::Arc;
use futures::{StreamExt, SinkExt}; 

use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::models::{LedgerEntry, DocId};
use crate::server::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("Client connected");
    
    // 1. Initial Load Removed (Client must OpenDoc)
    // 2. Subscribe to Broadcast
    let mut rx = state.tx.subscribe();
    
    // Split socket
    let (mut sender, mut receiver) = socket.split();
    
    // Task: Receive from Broadcast -> Send to Client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Filter: Don't echo back edits to the sender? 
            // Ideally we need ClientId. For now, we trust frontend diffing to handle echoes 
            // or we accept "re-applying" same state is no-op.
            // Actually, if we broadcast NewOps, the client will apply it.
            // If the client *just* sent it, it already applied it.
            // We need to differentiate.
            // Simplified: Verification first.
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Task: Receive from Client -> Persist -> Broadcast
    let state_clone = state.clone();
    let tx = state.tx.clone();
    
    while let Some(msg) = receiver.next().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            return;
        };

        if let Message::Text(text) = msg {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                match client_msg {
                     ClientMessage::Edit { doc_id, op, client_id } => {
                       let entry = LedgerEntry {
                           doc_id,
                           op: op.clone(),
                           timestamp: chrono::Utc::now().timestamp_millis(),
                       };
                       
                       match state_clone.ledger.append_op(&entry) {
                           Ok(seq) => {
                               // Broadcast to ALL with Sequence and ClientId
                               let _ = tx.send(ServerMessage::NewOp { 
                                   doc_id, 
                                   op, 
                                   seq,
                                   client_id 
                               });
                               
                               // [Persistence via SyncManager]
                               if let Err(e) = state_clone.sync_manager.persist_doc(doc_id) {
                                   tracing::error!("SyncManager failed to persist doc {}: {:?}", doc_id, e);
                               }
                           }
                           Err(e) => {
                               tracing::error!("Failed to persist op: {:?}", e);
                           }
                       }
                    }
                    ClientMessage::RequestHistory { doc_id } => {
                        // Fetch all ops
                        if let Ok(entries) = state_clone.ledger.get_ops(doc_id) {
                            let ops: Vec<(u64, deve_core::models::Op)> = entries.into_iter()
                                .map(|(seq, entry)| (seq, entry.op))
                                .collect();
                            
                            let msg = ServerMessage::History { doc_id, ops };
                            let _ = tx.send(msg);
                        }
                    }
                    ClientMessage::ListDocs => {
                         if let Ok(docs) = state_clone.ledger.list_docs() {
                             let msg = ServerMessage::DocList { docs };
                             let _ = tx.send(msg);
                         }
                    }
                     ClientMessage::OpenDoc { doc_id } => {
                          tracing::info!("OpenDoc Request for DocID: {}", doc_id);
                          
                          // [Reconciliation via SyncManager]
                          if let Err(e) = state_clone.sync_manager.reconcile_doc(doc_id) {
                              tracing::error!("SyncManager reconcile failed: {:?}", e);
                          }

                          // Return Snapshot from Ledger (Truth)
                          let entries_with_seq = state_clone.ledger.get_ops(doc_id).unwrap_or_default();
                          let ops: Vec<deve_core::models::LedgerEntry> = entries_with_seq.iter().map(|(_, entry)| entry.clone()).collect();
                          let final_content = deve_core::state::reconstruct_content(&ops);
                          let version = entries_with_seq.last().map(|(seq, _)| *seq).unwrap_or(0);
                          
                          let snapshot = ServerMessage::Snapshot { doc_id, content: final_content, version };
                          let _ = tx.send(snapshot);
                     }
                    ClientMessage::CreateDoc { name } => {
                        let mut filename = name.clone();
                        if !filename.ends_with(".md") {
                            filename.push_str(".md");
                        }
                        
                        // Prevent directory traversal (basic check)
                        if filename.contains("..") || filename.starts_with('/') || filename.contains('\\') {
                             tracing::error!("Invalid filename: {}", filename);
                             return; 
                        }
                            
                        let path = state_clone.vault_path.join(&filename);
                        
                        // Ensure parent directory exists
                        if let Some(parent) = path.parent() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                tracing::error!("Failed to create directories: {:?}", e);
                                return;
                            }
                        }

                        if path.exists() {
                             // Already exists? Just register/get ID
                             if let Ok(_doc_id) = state_clone.ledger.create_docid(&filename) {
                                  // Send updated list
                                  if let Ok(docs) = state_clone.ledger.list_docs() {
                                      let _ = tx.send(ServerMessage::DocList { docs });
                                  }
                             }
                        } else {
                             // New file: Write headers or empty
                             if let Err(e) = std::fs::write(&path, "# New Note\n") {
                                 tracing::error!("Failed to create file: {:?}", e);
                             } else {
                                 if let Ok(doc_id) = state_clone.ledger.create_docid(&filename) {
                                     // Success
                                     tracing::info!("Created doc: {} ({})", filename, doc_id);
                                     
                                     // Broadcast List Update
                                     if let Ok(docs) = state_clone.ledger.list_docs() {
                                         let _ = tx.send(ServerMessage::DocList { docs });
                                     }
                                 }
                             }
                        }
                    }
                    ClientMessage::RenameDoc { old_path, new_path } => {
                         // Basic rename impl: fs::rename + update ledger (not fully supported by ledger yet, may need raw updates)
                         // For MVP, since ledger tracks by ID and assumes static paths map to IDs, this is tricky.
                         // But we can just fs::rename and trigger list update. Ledger might get confused if it caches paths.
                         // Actually, our simple ledger might just re-scan or we assume ID checks path.
                         
                         let src = state_clone.vault_path.join(&old_path);
                         // Auto-append .md if missing for usability, unless folder? 
                         // Frontend sends full path. Let's assume frontend handles extensions or we do.
                         // For simplicity, assume files have .md. Folders don't.
                         // But we only show files in flat list? No, we have folders.
                         // Recursive rename is dangerous. Let's start with FILE rename.
                         
                         let mut dst_name = new_path.clone();
                         // Preserve extension if input lacks it and src has it?
                         if !dst_name.ends_with(".md") && old_path.ends_with(".md") {
                             dst_name.push_str(".md");
                         }
                         
                         let dst = state_clone.vault_path.join(&dst_name);
                         
                         if src.exists() {
                             if let Some(parent) = dst.parent() {
                                 let _ = std::fs::create_dir_all(parent);
                             }
                             
                             if let Err(e) = std::fs::rename(&src, &dst) {
                                 tracing::error!("Failed to rename {} to {}: {:?}", old_path, dst_name, e);
                             } else {
                                  tracing::info!("Renamed {} to {}", old_path, dst_name);
                                  
                                  // Update Ledger
                                  // Check if it was a directory (now at dst)
                                  if dst.is_dir() {
                                      if let Err(e) = state_clone.ledger.rename_folder(&old_path, &dst_name) {
                                          tracing::error!("Failed to update ledger rename folder: {:?}", e);
                                      }
                                  } else {
                                      if let Err(e) = state_clone.ledger.rename_doc(&old_path, &dst_name) {
                                          tracing::error!("Failed to update ledger rename doc: {:?}", e);
                                      }
                                  }

                                  if let Ok(docs) = state_clone.ledger.list_docs() {
                                      let _ = tx.send(ServerMessage::DocList { docs });
                                  }
                             }
                         } else {
                             tracing::warn!("Rename failed: Source does not exist: {:?}", src);
                         }
                    }
                    ClientMessage::DeleteDoc { path } => {
                        let target = state_clone.vault_path.join(&path);
                        if target.exists() {
                            // Check if directory
                             if target.is_dir() {
                                 if let Err(e) = std::fs::remove_dir_all(&target) {
                                     tracing::error!("Failed to delete dir {}: {:?}", path, e);
                                 } else {
                                     tracing::info!("Deleted dir {}", path);
                                 }
                             } else {
                                 // Trash bin logic? For now, hard delete.
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
                        if let Err(e) = state_clone.ledger.delete_doc(&path) {
                            tracing::error!("Failed to update ledger delete: {:?}", e);
                        }
                        
                        // Update List
                        if let Ok(docs) = state_clone.ledger.list_docs() {
                            let _ = tx.send(ServerMessage::DocList { docs });
                        }
                    }
                }
            }
        }
    }
    
    send_task.abort();
    tracing::info!("Client disconnected");
}

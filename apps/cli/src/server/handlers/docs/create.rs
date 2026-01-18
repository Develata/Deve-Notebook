// apps/cli/src/server/handlers/docs/create.rs
//! # 创建文档处理器

use super::validate_path;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
use deve_core::utils::path::join_normalized;
use std::sync::Arc;

/// 处理创建文档请求
///
/// **流程**:
/// 1. 校验文件名 (防止遍历攻击、深度超限)
/// 2. 确保父目录存在
/// 3. 创建文件并写入默认内容
/// 4. 在 Ledger 中注册 DocId
/// 5. 广播更新后的文档列表
pub async fn handle_create_doc(state: &Arc<AppState>, ch: &DualChannel, name: String) {
    // 1. 确保以 .md 结尾
    let mut filename = name.clone();
    if !filename.ends_with(".md") {
        filename.push_str(".md");
    }

    // 2. 路径校验
    if !validate_path(&filename, ch) {
        return;
    }

    // 3. 构建完整路径
    let path = join_normalized(&state.vault_path, &filename);

    // 4. 确保父目录存在
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::error!("创建目录失败: {:?}", e);
            ch.send_error(format!("Failed to create directories: {}", e));
            return;
        }
    }

    // 5. 创建文件或注册已存在文件
    if path.exists() {
        // 文件已存在，仅注册到 Ledger
        if let Ok(_doc_id) = state.repo.create_docid(&filename) {
            handle_list_docs(state, ch, None).await;
        }
    } else if let Err(e) = std::fs::write(&path, "# New Note\n") {
        tracing::error!("创建文件失败: {:?}", e);
        ch.send_error(format!("Failed to create file: {}", e));
    } else if let Ok(doc_id) = state.repo.create_docid(&filename) {
        tracing::info!("已创建文档: {} ({})", filename, doc_id);
        handle_list_docs(state, ch, None).await;
    }
}

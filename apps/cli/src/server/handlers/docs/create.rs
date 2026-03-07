// apps/cli/src/server/handlers/docs/create.rs
//! # 创建文档处理器

use super::create_file::handle_file_create;
use super::create_folder::handle_folder_create;
use super::validate_path;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::resolve_session_repo;
use crate::server::session::WsSession;
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
    session: &mut WsSession,
    name: String,
) {
    if session.is_readonly() {
        tracing::debug!("Create ignored: session is readonly (remote branch)");
        return;
    }
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_error(err.to_string());
            return;
        }
    };

    let filename = normalize_name(name);

    if !validate_path(&filename, ch) {
        return;
    }

    let path = join_normalized(&state.vault_path, &filename);

    if let Err(e) = ensure_parent_dirs(&path) {
        tracing::error!("创建目录失败: {:?}", e);
        ch.send_error(format!("Failed to create directories: {}", e));
        return;
    }

    if filename.ends_with('/') {
        handle_folder_create(state, ch, session, &scope, &path, &filename).await;
    } else {
        handle_file_create(state, ch, session, &scope, &path, &filename).await;
    }
}

fn normalize_name(mut name: String) -> String {
    if !name.ends_with('/') && !name.ends_with(".md") {
        name.push_str(".md");
    }
    name
}

fn ensure_parent_dirs(path: &std::path::Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

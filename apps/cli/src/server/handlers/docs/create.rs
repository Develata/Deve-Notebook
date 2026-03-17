// apps/cli/src/server/handlers/docs/create.rs
//! # 创建文档处理器

use super::create_file::handle_file_create;
use super::create_folder::handle_folder_create;
use super::errors;
use super::{validate_file_path, validate_folder_path};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{
    local_repo_path, map_repo_scope_error, resolve_session_repo_and_sync,
};
use crate::server::session::WsSession;
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
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    if session.is_readonly() {
        tracing::debug!("Create rejected: session is readonly (remote branch)");
        errors::remote_branch_readonly_scoped(ch, scope_nonce);
        return;
    }
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(err), scope_nonce);
            return;
        }
    };

    let filename = normalize_name(name);

    let valid = if filename.ends_with('/') {
        validate_folder_path(&filename, ch)
    } else {
        validate_file_path(&filename, ch)
    };
    if !valid {
        return;
    }

    let path = match local_repo_path(state, &scope, &filename) {
        Ok(path) => path,
        Err(err) => {
            errors::classified_failure_scoped(ch, err.to_string(), scope_nonce);
            return;
        }
    };

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

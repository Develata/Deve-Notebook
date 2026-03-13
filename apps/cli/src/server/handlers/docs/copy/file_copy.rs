use super::super::errors;
use super::prepare::CopyPaths;
use crate::server::handlers::docs::file_register::{
    broadcast_file_tree_update, create_file_from_content,
};
use deve_core::state;

use super::register::CopyRegisterCtx;

pub(super) fn copy_file(
    ctx: CopyRegisterCtx<'_>,
    paths: &CopyPaths,
    src_path: &str,
    dest_path: &str,
) -> bool {
    let Some(doc_id) = paths.src_doc_id else {
        errors::storage_not_found(ctx.ch, format!("Source doc missing: {}", src_path));
        return false;
    };
    let content = match ctx
        .state
        .repo
        .get_local_ops_in_local_repo(&ctx.scope.repo_name, doc_id)
    {
        Ok(ops) => {
            let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
            state::reconstruct_content(&entries)
        }
        Err(err) => {
            tracing::error!("读取复制源失败 {}: {:?}", src_path, err);
            errors::storage_persist_failed(ctx.ch, format!("Failed to load copy source: {}", err));
            return false;
        }
    };
    let doc_id =
        match create_file_from_content(ctx.state, ctx.scope, dest_path, &content, "local_copy") {
            Ok(doc_id) => doc_id,
            Err(err) => {
                tracing::error!("复制文档注册失败 {}: {:?}", dest_path, err);
                errors::storage_persist_failed(
                    ctx.ch,
                    format!("Failed to register copied file: {}", err),
                );
                return false;
            }
        };
    tracing::info!("已复制 {} -> {} (DocId: {})", src_path, dest_path, doc_id);
    broadcast_file_tree_update(ctx.state, ctx.ch, ctx.scope, doc_id, ctx.scope_nonce);
    true
}

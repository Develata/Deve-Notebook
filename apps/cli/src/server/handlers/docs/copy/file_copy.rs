use super::super::errors;
use super::prepare::CopyPaths;
use crate::server::handlers::docs::file_register::create_file_from_content;
use deve_core::state;

use super::register::CopyRegisterCtx;

pub(super) fn copy_file(
    ctx: CopyRegisterCtx<'_>,
    paths: &CopyPaths,
    src_path: &str,
    dest_path: &str,
) -> bool {
    let Some(doc_id) = paths.src_doc_id else {
        errors::storage_not_found_scoped(
            ctx.ch,
            format!("Source doc missing: {}", src_path),
            ctx.scope_nonce,
        );
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
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Failed to load copy source: {}", err),
                ctx.scope_nonce,
            );
            return false;
        }
    };
    let doc_id =
        match create_file_from_content(ctx.state, ctx.scope, dest_path, &content, "local_copy") {
            Ok(doc_id) => doc_id,
            Err(err) => {
                tracing::error!("复制文档注册失败 {}: {:?}", dest_path, err);
                errors::storage_persist_failed_scoped(
                    ctx.ch,
                    format!("Failed to register copied file: {}", err),
                    ctx.scope_nonce,
                );
                return false;
            }
        };
    tracing::info!("已复制 {} -> {} (DocId: {})", src_path, dest_path, doc_id);
    true
}

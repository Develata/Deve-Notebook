//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Copied markdown document registration.

use super::CopyRegisterCtx;
use super::path::map_dest_rel;
use crate::server::handlers::docs::copy_utils::collect_md_files;
use crate::server::handlers::docs::errors;
use crate::server::handlers::docs::file_register::create_file_from_content;
use deve_core::state;
use std::path::Path;

pub(super) fn register_files(
    ctx: CopyRegisterCtx<'_>,
    src: &Path,
    base: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    let files = match collect_md_files(src, base) {
        Ok(files) => files,
        Err(err) => {
            tracing::error!("收集 .md 文件失败: {:?}", err);
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Failed to collect copied files: {}", err),
                ctx.scope_nonce,
            );
            return false;
        }
    };
    let count = files.len();
    for rel_path in files {
        let dest_rel = match map_dest_rel(&rel_path, src_path, dest_path) {
            Ok(dest_rel) => dest_rel,
            Err(err) => {
                errors::storage_persist_failed_scoped(
                    ctx.ch,
                    format!("Failed to map copied file path: {}", err),
                    ctx.scope_nonce,
                );
                return false;
            }
        };
        if !register_file(ctx, &rel_path, &dest_rel) {
            return false;
        }
    }
    tracing::info!("目录复制完成: {} 下注册 {} 个文档", dest_path, count);
    true
}

fn register_file(ctx: CopyRegisterCtx<'_>, src_rel: &str, dest_rel: &str) -> bool {
    let doc_id = match ctx
        .state
        .repo
        .get_tracked_docid_in_local_repo(&ctx.scope.repo_name, src_rel)
    {
        Ok(Some(doc_id)) => doc_id,
        Ok(None) => {
            errors::storage_not_found_scoped(
                ctx.ch,
                format!("Source doc not tracked: {}", src_rel),
                ctx.scope_nonce,
            );
            return false;
        }
        Err(err) => {
            errors::classified_failure_scoped(
                ctx.ch,
                format!("Failed to resolve copied source: {}", err),
                ctx.scope_nonce,
            );
            return false;
        }
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
            tracing::error!("加载复制源失败 {}: {:?}", src_rel, err);
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Failed to load copied file: {}", err),
                ctx.scope_nonce,
            );
            return false;
        }
    };
    match create_file_from_content(ctx.state, ctx.scope, dest_rel, &content, "local_copy") {
        Ok((doc_id, _ops)) => {
            tracing::debug!(
                "注册复制文档: {} -> {} (DocId: {})",
                src_rel,
                dest_rel,
                doc_id
            );
            true
        }
        Err(err) => {
            tracing::error!("Ledger 注册失败 {}: {:?}", dest_rel, err);
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Failed to register copied file: {}", err),
                ctx.scope_nonce,
            );
            false
        }
    }
}

//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Copied directory structure registration.

use super::super::errors;
use super::CopyRegisterCtx;
use super::register_path::map_dest_rel;
use crate::server::handlers::docs::copy_utils::collect_dirs;
use std::path::Path;

pub(super) fn register_dirs(
    ctx: CopyRegisterCtx<'_>,
    src: &Path,
    base: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    let mut dirs = match collect_dirs(src, base) {
        Ok(dirs) => dirs,
        Err(err) => {
            tracing::error!("收集目录失败: {:?}", err);
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Failed to collect copied dirs: {}", err),
                ctx.scope_nonce,
            );
            return false;
        }
    };
    dirs.sort_by_key(|path| path.matches('/').count());
    for dir_path in dirs {
        if !register_dir(ctx, &dir_path, src_path, dest_path) {
            return false;
        }
    }
    true
}

fn register_dir(ctx: CopyRegisterCtx<'_>, dir_path: &str, src_path: &str, dest_path: &str) -> bool {
    let dest_rel = match map_dest_rel(dir_path, src_path, dest_path) {
        Ok(dest_rel) => dest_rel,
        Err(err) => {
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Failed to map copied dir path: {}", err),
                ctx.scope_nonce,
            );
            return false;
        }
    };
    match ctx.state.repo.apply_dir_create_structure_in_local_repo(
        &ctx.scope.repo_name,
        &dest_rel,
        "local_copy",
    ) {
        Ok(_) => true,
        Err(err) => {
            tracing::error!("目录节点创建失败: {:?}", err);
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Dir node creation failed: {}", err),
                ctx.scope_nonce,
            );
            false
        }
    }
}

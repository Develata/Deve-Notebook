use super::copy_dir_on_disk;
use crate::server::repo_scope::run_on_resolved_local_repo;
use std::path::Path;

use super::register::{CopyRegisterCtx, register_copied_docs};

pub(super) fn copy_dir(
    ctx: CopyRegisterCtx<'_>,
    src: &Path,
    dst: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    if !register_copied_docs(ctx, src, src_path, dest_path) {
        return false;
    }
    if !copy_dir_on_disk(ctx.ch, src, dst, src_path) {
        return false;
    }
    if let Ok(report) = run_on_resolved_local_repo(ctx.state, ctx.scope, |db| {
        deve_core::ledger::node_check::check_node_consistency(db)
    }) && !report.is_clean()
    {
        tracing::warn!(
            "Node consistency after copy: missing={} orphan={}",
            report.missing_nodes.len(),
            report.orphan_nodes.len()
        );
    }
    tracing::info!("已复制目录 {} -> {}", src_path, dest_path);
    true
}

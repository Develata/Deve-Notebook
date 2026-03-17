use super::super::errors;
use super::copy_dir_on_disk;
use crate::server::repo_scope::run_on_resolved_local_repo;
use deve_core::ledger::node_check::NodeConsistencyReport;
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
    if !copy_dir_on_disk(ctx.ch, src, dst, src_path, ctx.scope_nonce) {
        return false;
    }
    let report = match run_on_resolved_local_repo(ctx.state, ctx.scope, |db| {
        deve_core::ledger::node_check::check_node_consistency(db)
    }) {
        Ok(report) => report,
        Err(err) => {
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Failed to validate node consistency after copy: {}", err),
                ctx.scope_nonce,
            );
            return false;
        }
    };
    if !ensure_clean_node_consistency(ctx.ch, &report, ctx.scope_nonce) {
        return false;
    }
    tracing::info!("已复制目录 {} -> {}", src_path, dest_path);
    true
}

fn ensure_clean_node_consistency(
    ch: &crate::server::channel::DualChannel,
    report: &NodeConsistencyReport,
    scope_nonce: Option<u64>,
) -> bool {
    if report.is_clean() {
        return true;
    }
    errors::storage_persist_failed_scoped(
        ch,
        format!(
            "Node consistency dirty after copy: missing={} orphan={}",
            report.missing_nodes.len(),
            report.orphan_nodes.len()
        ),
        scope_nonce,
    );
    false
}

#[cfg(test)]
#[path = "dir_copy_test.rs"]
mod tests;

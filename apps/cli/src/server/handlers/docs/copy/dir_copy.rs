use super::copy_dir_on_disk;
use super::super::errors;
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
    if !copy_dir_on_disk(ctx.ch, src, dst, src_path) {
        return false;
    }
    let report = match run_on_resolved_local_repo(ctx.state, ctx.scope, |db| {
        deve_core::ledger::node_check::check_node_consistency(db)
    }) {
        Ok(report) => report,
        Err(err) => {
            errors::storage_persist_failed(
                ctx.ch,
                format!("Failed to validate node consistency after copy: {}", err),
            );
            return false;
        }
    };
    if !ensure_clean_node_consistency(ctx.ch, &report) {
        return false;
    }
    tracing::info!("已复制目录 {} -> {}", src_path, dest_path);
    true
}

fn ensure_clean_node_consistency(
    ch: &crate::server::channel::DualChannel,
    report: &NodeConsistencyReport,
) -> bool {
    if report.is_clean() {
        return true;
    }
    errors::storage_persist_failed(
        ch,
        format!(
            "Node consistency dirty after copy: missing={} orphan={}",
            report.missing_nodes.len(),
            report.orphan_nodes.len()
        ),
    );
    false
}

#[cfg(test)]
#[path = "dir_copy_test.rs"]
mod tests;

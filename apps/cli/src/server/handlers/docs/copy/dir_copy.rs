use super::copy_dir_on_disk;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{ResolvedRepo, run_on_resolved_local_repo};
use std::path::Path;
use std::sync::Arc;

use super::register::register_copied_docs;

pub(super) fn copy_dir(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    src: &Path,
    dst: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    if !register_copied_docs(state, ch, scope, src, src_path, dest_path) {
        return false;
    }
    if !copy_dir_on_disk(ch, src, dst, src_path) {
        return false;
    }
    if let Ok(report) = run_on_resolved_local_repo(state, scope, |db| {
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

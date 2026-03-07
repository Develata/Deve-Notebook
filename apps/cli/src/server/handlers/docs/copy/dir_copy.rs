use super::copy_dir_on_disk;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{ResolvedRepo, local_repo_path, run_on_resolved_local_repo};
use std::path::Path;
use std::sync::Arc;

use super::register::register_copied_docs;

pub(super) fn copy_dir(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    dst: &Path,
    src_path: &str,
    dest_path: &str,
) {
    let src = match local_repo_path(state, scope, src_path) {
        Ok(path) => path,
        Err(err) => {
            ch.send_error(err.to_string());
            return;
        }
    };
    if !copy_dir_on_disk(ch, &src, dst, src_path) {
        return;
    }
    register_copied_docs(state, ch, scope, dst, dest_path);
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
}

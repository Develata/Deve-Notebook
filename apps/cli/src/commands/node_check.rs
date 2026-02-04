// apps/cli/src/commands/node_check.rs
//! # Node 一致性检查命令

use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::ledger::node_check::{check_node_consistency, repair_missing_nodes};
use std::path::PathBuf;

pub fn run(ledger_dir: &PathBuf, snapshot_depth: usize, repair: bool) -> Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let report = repo.run_on_local_repo(repo.local_repo_name(), |db| {
        if repair {
            repair_missing_nodes(db)
        } else {
            check_node_consistency(db)
        }
    })?;

    println!(
        "node_check: missing_nodes={} orphan_nodes={}",
        report.missing_nodes.len(),
        report.orphan_nodes.len()
    );

    if !report.missing_nodes.is_empty() {
        println!("missing_nodes:");
        for (doc_id, path) in report.missing_nodes {
            println!("  {} {}", doc_id, path);
        }
    }

    if !report.orphan_nodes.is_empty() {
        println!("orphan_nodes:");
        for (node_id, path) in report.orphan_nodes {
            println!("  {} {}", node_id, path);
        }
    }

    Ok(())
}

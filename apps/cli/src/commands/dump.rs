// apps/cli/src/commands/dump.rs
//! # Dump 命令 (调试用)
//!
//! 打印指定文档的所有操作历史并重建内容

use deve_core::ledger::RepoManager;
use std::path::PathBuf;

/// 转储命令 (调试用)
///
/// **功能**:
/// 打印指定文档的所有操作历史 (Ops)。
/// 并尝试重建文档内容以验证正确性。
pub fn run(
    ledger_dir: &PathBuf,
    path_str: String,
    repo_name: Option<String>,
    snapshot_depth: usize,
) -> anyhow::Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_name = repo.resolve_local_repo_name(None, repo_name.as_deref())?;
    if let Some(doc_id) = repo.get_docid_in_local_repo(&repo_name, &path_str)? {
        println!("DocId: {}", doc_id);
        let ops = repo.get_local_ops_in_local_repo(&repo_name, doc_id)?;
        println!("Found {} ops:", ops.len());
        for (i, (seq, entry)) in ops.iter().enumerate() {
            println!("[{}] Seq:{} {} {:?}", i, seq, entry.timestamp, entry.op);
        }

        let ops_vec: Vec<deve_core::models::LedgerEntry> =
            ops.iter().map(|(_, e)| e.clone()).collect();
        let content = deve_core::state::reconstruct_content(&ops_vec);
        println!("\nReconstructed Content:\n---\n{}\n---", content);
    } else {
        println!("Path not found in Ledger.");
    }
    Ok(())
}

use std::path::PathBuf;
use deve_core::ledger::RepoManager;
use anyhow::Result;

/// 转储命令 (调试用)
///
/// **功能**:
/// 打印指定文档的所有操作历史 (Ops)。
/// 并尝试重建文档内容以验证正确性。
pub fn run(ledger_dir: &PathBuf, path_str: String, snapshot_depth: usize) -> anyhow::Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None)?;
    if let Some(doc_id) = repo.get_docid(&path_str)? {
        println!("DocId: {}", doc_id);
        let ops = repo.get_local_ops(doc_id)?;
        println!("Found {} ops:", ops.len());
        for (i, (seq, entry)) in ops.iter().enumerate() {
            println!("[{}] Seq:{} {} {:?}", i, seq, entry.timestamp, entry.op);
        }
        
        let ops_vec: Vec<deve_core::models::LedgerEntry> = ops.iter().map(|(_, e)| e.clone()).collect();
        let content = deve_core::state::reconstruct_content(&ops_vec);
        println!("\nReconstructed Content:\n---\n{}\n---", content);
    } else {
        println!("Path not found in Ledger.");
    }
    Ok(())
}

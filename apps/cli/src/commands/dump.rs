// apps/cli/src/commands/dump.rs
//! # Dump 命令 (调试用)
//!
//! 打印指定文档的所有操作历史并重建内容

use crate::admin_api::DumpResponse;
use crate::commands::live_proxy;
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
    let repo = match RepoManager::init(ledger_dir, snapshot_depth, None, None) {
        Ok(repo) => repo,
        Err(err) if live_proxy::is_db_lock_error(&err) => {
            let dump = live_proxy::dump(ledger_dir, &path_str, repo_name.as_deref())?;
            return print_dump(&dump);
        }
        Err(err) => return Err(err),
    };
    let repo_name = repo.resolve_local_repo_name(None, repo_name.as_deref())?;
    if let Some(doc_id) = repo.get_docid_in_local_repo(&repo_name, &path_str)? {
        let ops = repo.get_local_ops_in_local_repo(&repo_name, doc_id)?;
        let dump = DumpResponse {
            doc_id: Some(doc_id),
            node_id: Some(deve_core::models::NodeId::from_doc_id(doc_id)),
            node_meta: None,
            content: deve_core::state::reconstruct_content(
                &ops.iter().map(|(_, e)| e.clone()).collect::<Vec<_>>(),
            ),
            ops,
            structure_ops: Vec::new(),
        };
        print_dump(&dump)?;
    } else {
        println!("Path not found in Ledger.");
    }
    Ok(())
}

fn print_dump(dump: &DumpResponse) -> anyhow::Result<()> {
    println!("NodeId: {:?}", dump.node_id);
    println!("DocId: {:?}", dump.doc_id);
    println!("NodeMeta: {:?}", dump.node_meta);
    println!("Found {} structure ops:", dump.structure_ops.len());
    for (index, (seq, entry)) in dump.structure_ops.iter().enumerate() {
        println!(
            "[S{}] Seq:{} {} {:?}",
            index, seq, entry.timestamp, entry.event
        );
    }
    println!("Found {} content ops:", dump.ops.len());
    for (index, (seq, entry)) in dump.ops.iter().enumerate() {
        println!(
            "[C{}] Seq:{} {} {:?}",
            index,
            seq,
            entry.timestamp,
            entry.content_op()
        );
    }
    println!("\nReconstructed Content:\n---\n{}\n---", dump.content);
    Ok(())
}

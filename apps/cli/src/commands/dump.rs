// apps/cli/src/commands/dump.rs
//! # Dump 命令 (调试用)
//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 10_rendering#document-authority-bridge
//!
//! 打印指定文档的所有操作历史并重建内容

use crate::admin_api::DumpResponse;
use crate::commands::live_proxy;
use crate::commands::repo_arg::resolve_local_repo_arg;
use crate::dump_support;
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
    let repo_name = resolve_local_repo_arg(&repo, repo_name.as_deref())?;
    match dump_support::build_dump(&repo, &repo_name, &path_str)? {
        Some(dump) => print_dump(&dump)?,
        None => println!("Path not found in Ledger."),
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

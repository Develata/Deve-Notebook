// apps\cli\src\commands
use crate::admin_api::ExportEntry;
use crate::commands::live_proxy;
use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::models::LedgerEntry;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// 导出命令
///
/// **功能**:
/// 将整个 Ledger 的数据导出为 JSON 格式 (Line-delimited JSON)。
/// 每个文档一行 JSON 对象。
///
/// **用途**:
/// 数据备份、迁移或分析。
pub fn run(
    ledger_dir: &PathBuf,
    output: Option<String>,
    repo_name: Option<String>,
    snapshot_depth: usize,
) -> Result<()> {
    let repo = match RepoManager::init(ledger_dir, snapshot_depth, None, None) {
        Ok(repo) => repo,
        Err(err) if live_proxy::is_db_lock_error(&err) => {
            let entries = live_proxy::export(ledger_dir, repo_name.as_deref())?;
            return write_entries(output, &entries);
        }
        Err(err) => return Err(err),
    };
    let repo_name = repo.resolve_local_repo_name(None, repo_name.as_deref())?;
    let docs = repo.list_local_docs(Some(&repo_name))?;
    let mut entries = Vec::with_capacity(docs.len());

    for (doc_id, path) in docs {
        let ops_with_seq = repo.get_local_ops_in_local_repo(&repo_name, doc_id)?;
        let ops: Vec<LedgerEntry> = ops_with_seq.into_iter().map(|(_, op)| op).collect();
        entries.push(ExportEntry { doc_id, path, ops });
    }

    write_entries(output, &entries)
}

fn write_entries(output: Option<String>, entries: &[ExportEntry]) -> Result<()> {
    let mut writer: Box<dyn Write> = if let Some(path) = output {
        let file = File::create(path)?;
        Box::new(BufWriter::new(file))
    } else {
        Box::new(std::io::stdout())
    };
    for entry in entries {
        writeln!(writer, "{}", serde_json::to_string(entry)?)?;
    }
    Ok(())
}

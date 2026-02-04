// apps\cli\src\commands
use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{DocId, LedgerEntry, RepoType};
use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// 导出条目结构
/// 用于序列化为 JSON 格式。
#[derive(Serialize)]
struct ExportEntry {
    doc_id: DocId,
    path: String,
    ops: Vec<LedgerEntry>,
}

/// 导出命令
///
/// **功能**:
/// 将整个 Ledger 的数据导出为 JSON 格式 (Line-delimited JSON)。
/// 每个文档一行 JSON 对象。
///
/// **用途**:
/// 数据备份、迁移或分析。
pub fn run(ledger_dir: &PathBuf, output: Option<String>, snapshot_depth: usize) -> Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let docs = repo.list_docs(&RepoType::Local(uuid::Uuid::nil()))?;

    let mut writer: Box<dyn Write> = if let Some(path) = output {
        let file = File::create(path)?;
        Box::new(BufWriter::new(file))
    } else {
        Box::new(std::io::stdout())
    };

    for (doc_id, path) in docs {
        let ops_with_seq = repo.get_local_ops(doc_id)?;
        let ops: Vec<LedgerEntry> = ops_with_seq.into_iter().map(|(_, op)| op).collect();

        let entry = ExportEntry { doc_id, path, ops };

        let json = serde_json::to_string(&entry)?;
        writeln!(writer, "{}", json)?;
    }

    Ok(())
}

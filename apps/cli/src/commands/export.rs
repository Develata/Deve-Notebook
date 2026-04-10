//! plan_ref:
//!   - 04_storage#backup-export
//!
//! Ledger → JSON Lines / Markdown exporter. JSONL path implements the
//! `MUST` disaster-recovery export guarantee: every authoritative fact in
//! `LEDGER_OPS` is emitted as one line, global_seq monotonic, via either
//! direct DB access or live-proxy fallback when the server holds the lock.

#[path = "export_doc.rs"]
mod export_doc;
#[cfg(test)]
#[path = "export_test.rs"]
mod tests;

use crate::admin_api::ExportEntry;
use crate::commands::live_proxy;
use crate::commands::repo_arg::resolve_local_repo_arg;
use crate::export_entries;
use anyhow::{Result, bail};
use deve_core::ledger::RepoManager;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// 导出命令
///
/// **功能**:
/// - `json` (默认): 将 Ledger facts 导出为 Line-delimited JSON。
/// - `markdown`: 从 Ledger 重建文档内容并写入输出目录。
pub fn run(
    ledger_dir: &PathBuf,
    output: Option<String>,
    repo_name: Option<String>,
    doc: Option<String>,
    snapshot_depth: usize,
    format: &str,
) -> Result<()> {
    match format {
        "json" => run_json(ledger_dir, output, repo_name, doc, snapshot_depth),
        "markdown" | "md" => run_markdown(ledger_dir, output, repo_name, doc, snapshot_depth),
        _ => bail!(
            "Unsupported export format: {}. Use 'json' or 'markdown'.",
            format
        ),
    }
}

fn run_json(
    ledger_dir: &PathBuf,
    output: Option<String>,
    repo_name: Option<String>,
    doc: Option<String>,
    snapshot_depth: usize,
) -> Result<()> {
    if doc.is_some() {
        bail!("JSON export does not support --doc; export is repo-scoped");
    }
    let repo = match RepoManager::init(ledger_dir, snapshot_depth, None, None) {
        Ok(repo) => repo,
        Err(err) if live_proxy::is_db_lock_error(&err) => {
            let entries = live_proxy::export(ledger_dir, repo_name.as_deref())?;
            return write_entries(output, &entries);
        }
        Err(err) => return Err(err),
    };
    let repo_name = resolve_local_repo_arg(&repo, repo_name.as_deref())?;
    write_entries(output, &export_entries::build(&repo, &repo_name)?)
}

fn run_markdown(
    ledger_dir: &PathBuf,
    output: Option<String>,
    repo_name: Option<String>,
    doc: Option<String>,
    snapshot_depth: usize,
) -> Result<()> {
    let repo = match RepoManager::init(ledger_dir, snapshot_depth, None, None) {
        Ok(repo) => repo,
        Err(err) if live_proxy::is_db_lock_error(&err) => {
            bail!(
                "Markdown export requires direct DB access, but the database is locked by a \
                 running serve process. Stop the server first, or use 'deve export --format json' \
                 which supports live proxy fallback."
            );
        }
        Err(err) => return Err(err),
    };
    let repo_name = resolve_local_repo_arg(&repo, repo_name.as_deref())?;
    if let Some(doc) = doc {
        return export_doc::export_markdown_doc(
            &repo,
            &repo_name,
            export_doc::parse_doc_id(&doc)?,
            export_doc::output_file(output)?,
        );
    }
    let output_dir = PathBuf::from(output.unwrap_or_else(|| "export".into()));
    let exported = export_doc::export_repo_markdown(&repo, &repo_name, &output_dir)?;
    println!("Exported {} markdown files to {:?}", exported, output_dir);
    Ok(())
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

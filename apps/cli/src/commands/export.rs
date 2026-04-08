//! plan_ref:
//!   - 04_storage.md §Data Integrity & Recovery (Disaster Recovery)
//!
//! Ledger → JSON Lines / Markdown exporter. JSONL path implements the
//! `MUST` disaster-recovery export guarantee: every authoritative fact in
//! `LEDGER_OPS` is emitted as one line, global_seq monotonic, via either
//! direct DB access or live-proxy fallback when the server holds the lock.

use crate::admin_api::ExportEntry;
use crate::commands::live_proxy;
use crate::commands::repo_arg::resolve_local_repo_arg;
use crate::export_entries;
use anyhow::{Context, Result, bail};
use deve_core::ledger::RepoManager;
use deve_core::ledger::metadata;
use deve_core::sync::rebuild;
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
    snapshot_depth: usize,
    format: &str,
) -> Result<()> {
    match format {
        "json" => run_json(ledger_dir, output, repo_name, snapshot_depth),
        "markdown" | "md" => run_markdown(ledger_dir, output, repo_name, snapshot_depth),
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
    let repo_name = resolve_local_repo_arg(&repo, repo_name.as_deref())?;
    write_entries(output, &export_entries::build(&repo, &repo_name)?)
}

fn run_markdown(
    ledger_dir: &PathBuf,
    output: Option<String>,
    repo_name: Option<String>,
    snapshot_depth: usize,
) -> Result<()> {
    let output_dir = PathBuf::from(output.unwrap_or_else(|| "export".into()));
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
    let docs = repo.run_on_local_repo(&repo_name, metadata::list_docs)?;

    let mut exported = 0u32;
    for (doc_id, path) in docs {
        if path.is_empty() {
            continue;
        }
        let result = rebuild::rebuild_local_doc_in_repo(&repo, &repo_name, doc_id)
            .with_context(|| format!("Failed to rebuild {}", path))?;

        let target = output_dir.join(&path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create dir for {}", path))?;
        }
        std::fs::write(&target, &result.content)
            .with_context(|| format!("Failed to write {}", path))?;
        exported += 1;
    }

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

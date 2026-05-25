//! plan_ref:
//!   - 03_storage/repair#backup-export
//!   - 04_repository#tree-projection-contract
//!
//! Ledger → JSON Lines / Markdown exporter. JSONL path implements the
//! `MUST` disaster-recovery export guarantee: every authoritative fact in
//! `LEDGER_OPS` is emitted as one line, global_seq monotonic, via either
//! direct DB access or live-proxy fallback when the server holds the lock.

mod doc;
#[cfg(test)]
mod tests;

use crate::admin_api::ExportEntry;
use crate::commands::live_proxy;
use crate::commands::repo_arg::resolve_local_repo_arg;
use crate::export_entries;
use anyhow::{Result, bail};
use deve_core::ledger::RepoManager;
use deve_core::sync::{ProjectionDiagnosticStatus, diagnose_projection_local_repo};
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
    allow_degraded_projection: bool,
) -> Result<()> {
    match format {
        "json" => run_json(ledger_dir, output, repo_name, doc, snapshot_depth),
        "markdown" | "md" => run_markdown(
            ledger_dir,
            output,
            repo_name,
            doc,
            snapshot_depth,
            allow_degraded_projection,
        ),
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
    allow_degraded_projection: bool,
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
    guard_markdown_projection(&repo, &repo_name, allow_degraded_projection)?;
    if let Some(doc) = doc {
        return doc::export_markdown_doc(
            &repo,
            &repo_name,
            doc::parse_doc_id(&doc)?,
            doc::output_file(output)?,
            allow_degraded_projection,
        );
    }
    let output_dir = PathBuf::from(output.unwrap_or_else(|| "export".into()));
    let exported = doc::export_repo_markdown(&repo, &repo_name, &output_dir)?;
    println!("Exported {} markdown files to {:?}", exported, output_dir);
    Ok(())
}

fn guard_markdown_projection(
    repo: &RepoManager,
    repo_name: &str,
    allow_degraded_projection: bool,
) -> Result<()> {
    let diagnostic = diagnose_projection_local_repo(repo, repo_name)?;
    if diagnostic.status != ProjectionDiagnosticStatus::AuthorityCorrupt {
        return Ok(());
    }
    let detail = diagnostic
        .issue
        .map(|issue| format!("{}: {}", issue.code, issue.detail))
        .unwrap_or_else(|| "unknown Structure Facts authority corruption".to_string());
    if !allow_degraded_projection {
        bail!(
            "Markdown export for repo {repo_name} requires healthy Structure Facts authority; \
             detected {detail}. Use --allow-degraded-projection to export from metadata fallback, \
             or use --format json for raw ledger facts."
        );
    }
    eprintln!(
        "warning: exporting repo {repo_name} from degraded metadata projection fallback: {detail}"
    );
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

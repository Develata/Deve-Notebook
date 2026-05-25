//! plan_ref:
//!   - 03_storage/repair#backup-export
//!   - 04_repository#tree-projection-contract

use anyhow::{Context, Result, anyhow};
use deve_core::ledger::{RepoManager, metadata};
use deve_core::models::DocId;
use deve_core::sync::rebuild;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(super) fn parse_doc_id(raw: &str) -> Result<DocId> {
    Ok(DocId(
        Uuid::parse_str(raw).with_context(|| format!("Invalid doc id: {raw}"))?,
    ))
}

pub(super) fn output_file(output: Option<String>) -> Result<PathBuf> {
    output
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("Markdown export for a single doc requires --output/--out"))
}

pub(super) fn write_markdown_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dir for {}", path.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub(super) fn export_repo_markdown(
    repo: &RepoManager,
    repo_name: &str,
    output_dir: &Path,
) -> Result<u32> {
    let docs = repo.run_on_local_repo(repo_name, metadata::list_docs)?;
    let mut exported = 0u32;
    for (doc_id, path) in docs {
        if path.is_empty() {
            continue;
        }
        let result = rebuild::rebuild_local_doc_in_repo(repo, repo_name, doc_id)
            .with_context(|| format!("Failed to rebuild {}", path))?;
        write_markdown_file(&output_dir.join(&path), &result.content)?;
        exported += 1;
    }
    Ok(exported)
}

pub(super) fn export_markdown_doc(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
    output: PathBuf,
    allow_degraded_projection: bool,
) -> Result<()> {
    let path = resolve_export_doc_path(repo, repo_name, doc_id, allow_degraded_projection)?;
    let result = rebuild::rebuild_local_doc_in_repo(repo, repo_name, doc_id)
        .with_context(|| format!("Failed to rebuild {}", path))?;
    write_markdown_file(&output, &result.content)?;
    println!("Exported markdown {} to {:?}", path, output);
    Ok(())
}

fn resolve_export_doc_path(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
    allow_degraded_projection: bool,
) -> Result<String> {
    if let Some(meta) = repo.get_file_meta_for_doc_in_local_repo(repo_name, doc_id)? {
        return Ok(meta.path);
    }
    if allow_degraded_projection
        && let Some((_, path)) = repo
            .run_on_local_repo(repo_name, metadata::list_docs)?
            .into_iter()
            .find(|(candidate, _)| *candidate == doc_id)
    {
        return Ok(path);
    }
    Err(anyhow!(
        "Document not found in repo {}: {}",
        repo_name,
        doc_id
    ))
}

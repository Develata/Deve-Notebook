//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 14_tech_stack#graph-visualization
//!
//! Read-only graph projection export. This adapter gathers repo-scoped
//! document projections, then delegates all link parsing to `deve_core::graph`.

#[cfg(test)]
#[path = "graph_test.rs"]
mod tests;

use crate::commands::repo_arg::resolve_local_repo_arg;
use anyhow::{Context, Result, bail};
use deve_core::graph::{GraphDocument, GraphProjection, project_documents};
use deve_core::ledger::{RepoManager, metadata};
use deve_core::sync::{ProjectionDiagnosticStatus, diagnose_projection_local_repo, rebuild};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub fn run(
    ledger_dir: &Path,
    target_repo: Option<&str>,
    output: Option<String>,
    pretty: bool,
    allow_degraded_projection: bool,
    snapshot_depth: usize,
) -> Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_name = resolve_local_repo_arg(&repo, target_repo)?;
    guard_graph_projection(&repo, &repo_name, allow_degraded_projection)?;
    let projection = collect_repo_graph_projection(&repo, &repo_name)?;
    write_projection(output.as_deref(), pretty, &projection)
}

fn collect_repo_graph_projection(repo: &RepoManager, repo_name: &str) -> Result<GraphProjection> {
    let docs = collect_graph_documents(repo, repo_name)?;
    Ok(project_documents(&docs))
}

fn collect_graph_documents(repo: &RepoManager, repo_name: &str) -> Result<Vec<GraphDocument>> {
    let docs = repo.run_on_local_repo(repo_name, metadata::list_docs)?;
    let mut graph_docs = Vec::with_capacity(docs.len());
    for (doc_id, path) in docs {
        if path.is_empty() {
            continue;
        }
        let rebuilt = rebuild::rebuild_local_doc_in_repo(repo, repo_name, doc_id)
            .with_context(|| format!("Failed to rebuild graph source {}", path))?;
        graph_docs.push(GraphDocument {
            doc_id,
            path,
            content: rebuilt.content,
        });
    }
    graph_docs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(graph_docs)
}

fn guard_graph_projection(
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
            "Graph projection for repo {repo_name} requires healthy Structure Facts authority; \
             detected {detail}. Use --allow-degraded-projection to export from metadata fallback."
        );
    }
    eprintln!(
        "warning: exporting graph for repo {repo_name} from degraded metadata projection fallback: {detail}"
    );
    Ok(())
}

fn write_projection(
    output: Option<&str>,
    pretty: bool,
    projection: &GraphProjection,
) -> Result<()> {
    let mut writer: Box<dyn Write> = match output {
        Some(path) => {
            Box::new(BufWriter::new(File::create(path).with_context(|| {
                format!("Failed to create graph output {path}")
            })?))
        }
        None => Box::new(BufWriter::new(std::io::stdout())),
    };
    if pretty {
        serde_json::to_writer_pretty(&mut writer, projection)?;
    } else {
        serde_json::to_writer(&mut writer, projection)?;
    }
    writeln!(writer)?;
    Ok(())
}

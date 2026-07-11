//! plan_ref:
//!   - 17_tech_stack#graph-visualization
//!   - 04_repository#repo-scope-runtime
//!
//! Shared read-only graph projection adapter for CLI and protected HTTP.

use anyhow::{Context, Result};
use deve_core::graph::{GraphDocument, GraphProjection, project_documents};
use deve_core::ledger::RepoManager;
use deve_core::ledger::metadata;
use deve_core::ledger::traits::RepoSelector;
use deve_core::sync::{ProjectionDiagnosticStatus, diagnose_projection_local_repo, rebuild};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub(crate) enum GraphProjectionError {
    DegradedProjectionRequired { repo_name: String, detail: String },
}

impl fmt::Display for GraphProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphProjectionError::DegradedProjectionRequired { repo_name, detail } => write!(
                f,
                "Graph projection for repo {repo_name} requires healthy Structure Facts authority; \
                 detected {detail}. Use --allow-degraded-projection to export from metadata fallback."
            ),
        }
    }
}

impl Error for GraphProjectionError {}

pub(crate) fn is_degraded_projection_required(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| match cause.downcast_ref::<GraphProjectionError>() {
            Some(GraphProjectionError::DegradedProjectionRequired { .. }) => true,
            None => false,
        })
}

pub(crate) fn project_repo_graph(
    repo: &RepoManager,
    selector: &RepoSelector,
    allow_degraded_projection: bool,
) -> Result<GraphProjection> {
    let repo_name = repo
        .resolve_local_repo_name_for_execution(selector.repo_id, selector.repo_name.as_deref())?;
    guard_graph_projection(repo, &repo_name, allow_degraded_projection)?;
    let docs = collect_graph_documents(repo, &repo_name)?;
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
        return Err(GraphProjectionError::DegradedProjectionRequired {
            repo_name: repo_name.to_string(),
            detail,
        }
        .into());
    }
    eprintln!(
        "warning: exporting graph for repo {repo_name} from degraded metadata projection fallback: {detail}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{GraphProjectionError, is_degraded_projection_required, project_repo_graph};
    use anyhow::{Context as _, anyhow};
    use deve_core::ledger::RepoManager;
    use deve_core::ledger::traits::RepoSelector;
    use deve_core::models::{FactActor, Op};
    use tempfile::TempDir;

    fn seed_doc(repo: &RepoManager, path: &str, content: &str) -> deve_core::models::DocId {
        let (doc_id, _ops) = repo
            .apply_file_structure_in_local_repo(repo.local_repo_name(), path, None, "test")
            .expect("structure");
        repo.local_fact_writer(FactActor::new("test").expect("actor"))
            .append_content_in_local_repo(
                repo.local_repo_name(),
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
            )
            .expect("append op");
        doc_id
    }

    #[test]
    fn shared_graph_projection_resolves_repo_links_read_only() {
        let dir = TempDir::new().expect("tempdir");
        let repo = RepoManager::init(dir.path().join("ledger"), 8, None, None).expect("init repo");
        let target = seed_doc(&repo, "notes/target.md", "");
        seed_doc(&repo, "notes/source.md", "[[target]]");

        let graph =
            project_repo_graph(&repo, &RepoSelector::default(), false).expect("graph projection");

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].to_doc_id, target);
        assert!(graph.unresolved_links.is_empty());
    }

    #[test]
    fn degraded_projection_error_is_structurally_detectable() {
        let error: anyhow::Error = GraphProjectionError::DegradedProjectionRequired {
            repo_name: "default".into(),
            detail: "missing_parent: orphan".into(),
        }
        .into();

        assert!(is_degraded_projection_required(&error));
        assert!(error.to_string().contains("--allow-degraded-projection"));
        assert!(!is_degraded_projection_required(&anyhow!(
            "other graph error"
        )));

        let wrapped_error = Err::<(), _>(GraphProjectionError::DegradedProjectionRequired {
            repo_name: "default".into(),
            detail: "missing_parent: orphan".into(),
        })
        .context("HTTP graph projection failed")
        .expect_err("wrapped graph projection error");
        assert!(is_degraded_projection_required(&wrapped_error));
    }
}

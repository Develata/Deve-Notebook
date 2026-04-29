//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 14_tech_stack#graph-visualization
//!
//! Read-only graph projection export. This adapter gathers repo-scoped
//! document projections, then delegates all link parsing to `deve_core::graph`.

#[cfg(test)]
#[path = "graph_test.rs"]
mod tests;

use crate::graph_projection::project_repo_graph;
use anyhow::{Context, Result};
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
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
    let selector = RepoSelector {
        repo_id: None,
        repo_name: target_repo.map(str::to_owned),
    };
    let projection = project_repo_graph(&repo, &selector, allow_degraded_projection)?;
    write_projection(output.as_deref(), pretty, &projection)
}

fn write_projection(
    output: Option<&str>,
    pretty: bool,
    projection: &deve_core::graph::GraphProjection,
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

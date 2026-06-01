//! plan_ref:
//!   - 22_reliability_observability#observation-to-health-mapping
//!   - 04_repository#repo-health-and-repair
//!
//! Host-local durable journal for recoverable projection faults.

use crate::ledger::RepoManager;
use crate::models::{DocId, RepoId};
use crate::utils::{notegit, path::path_to_forward_slash};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard};

const JOURNAL_VERSION: u32 = 1;
const JOURNAL_FILE: &str = "projection-faults.toml";
const MAX_ERROR_CHARS: usize = 4096;

static JOURNAL_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ProjectionFaultKind {
    ProjectionWritebackFailed,
    ProjectionRebuildInterrupted,
}

pub(super) struct ProjectionFaultInput<'a> {
    pub(super) fault_kind: ProjectionFaultKind,
    pub(super) target_path: Option<&'a str>,
    pub(super) source_path: Option<&'a str>,
    pub(super) doc_id: Option<DocId>,
    pub(super) ledger_seq_or_head: Option<u64>,
    pub(super) last_error: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProjectionFaultJournal {
    version: u32,
    #[serde(default)]
    faults: Vec<DurableProjectionFault>,
}

impl Default for ProjectionFaultJournal {
    fn default() -> Self {
        Self {
            version: JOURNAL_VERSION,
            faults: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DurableProjectionFault {
    repo_id: RepoId,
    repo_name_at_fault: String,
    #[serde(default)]
    name_epoch: Option<u64>,
    fault_kind: ProjectionFaultKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    doc_id: Option<DocId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ledger_seq_or_head: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    projection_workspace_root: Option<String>,
    first_seen_at_unix_ms: i64,
    last_seen_at_unix_ms: i64,
    last_error: String,
    retry_count: u32,
    status: ProjectionFaultStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProjectionFaultStatus {
    Pending,
}

pub(super) fn journal_path(repo: &RepoManager) -> PathBuf {
    notegit::host_dir(repo.ledger_dir()).join(JOURNAL_FILE)
}

pub(super) fn record_fault(
    repo: &RepoManager,
    repo_name: &str,
    input: ProjectionFaultInput<'_>,
) -> Result<()> {
    let info = repo
        .get_repo_info_for(None, Some(repo_name))?
        .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", repo_name))?;
    let target_path = input.target_path.map(crate::utils::path::to_forward_slash);
    let source_path = input.source_path.map(crate::utils::path::to_forward_slash);
    let projection_workspace_root = repo
        .local_repo_workspace_root(repo_name)
        .ok()
        .map(|path| path_to_forward_slash(&path));
    let now = chrono::Utc::now().timestamp_millis();

    let _guard = lock_journal()?;
    let path = journal_path(repo);
    let mut journal = read_journal_file(&path)?;
    if let Some(fault) = journal.faults.iter_mut().find(|fault| {
        fault.status == ProjectionFaultStatus::Pending
            && fault.repo_id == info.uuid
            && fault.fault_kind == input.fault_kind
            && fault.target_path == target_path
            && fault.source_path == source_path
            && fault.doc_id == input.doc_id
    }) {
        fault.repo_name_at_fault = info.name;
        fault.ledger_seq_or_head = input.ledger_seq_or_head;
        fault.projection_workspace_root = projection_workspace_root;
        fault.last_seen_at_unix_ms = now;
        fault.last_error = bounded_error(input.last_error);
        fault.retry_count = fault.retry_count.saturating_add(1);
    } else {
        journal.faults.push(DurableProjectionFault {
            repo_id: info.uuid,
            repo_name_at_fault: info.name,
            name_epoch: None,
            fault_kind: input.fault_kind,
            target_path,
            source_path,
            doc_id: input.doc_id,
            ledger_seq_or_head: input.ledger_seq_or_head,
            projection_workspace_root,
            first_seen_at_unix_ms: now,
            last_seen_at_unix_ms: now,
            last_error: bounded_error(input.last_error),
            retry_count: 1,
            status: ProjectionFaultStatus::Pending,
        });
    }
    write_journal_file(&path, &journal)
}

pub(super) fn load_degraded_repo_names(repo: &RepoManager) -> Result<Vec<String>> {
    let _guard = lock_journal()?;
    let journal = read_journal_file(&journal_path(repo))?;
    let mut repo_names = Vec::new();
    for fault in journal
        .faults
        .iter()
        .filter(|fault| fault.status == ProjectionFaultStatus::Pending)
    {
        match repo
            .repo_scope_runtime()
            .resolve_local_repo_name_for_execution(Some(fault.repo_id), None)
        {
            Ok(repo_name) => repo_names.push(repo_name),
            Err(err) => tracing::warn!(
                repo_id = %fault.repo_id,
                repo_name_at_fault = %fault.repo_name_at_fault,
                error = %err,
                "Ignoring durable projection fault for unknown or invalid local repo"
            ),
        }
    }
    repo_names.sort();
    repo_names.dedup();
    Ok(repo_names)
}

pub(super) fn clear_faults_for_repo(repo: &RepoManager, repo_name: &str) -> Result<()> {
    let info = repo
        .get_repo_info_for(None, Some(repo_name))?
        .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", repo_name))?;

    let _guard = lock_journal()?;
    let path = journal_path(repo);
    let mut journal = read_journal_file(&path)?;
    journal.faults.retain(|fault| fault.repo_id != info.uuid);
    write_journal_file(&path, &journal)
}

fn lock_journal() -> Result<MutexGuard<'static, ()>> {
    JOURNAL_LOCK
        .lock()
        .map_err(|_| anyhow!("Projection fault journal lock is poisoned"))
}

fn read_journal_file(path: &Path) -> Result<ProjectionFaultJournal> {
    if !path
        .try_exists()
        .with_context(|| format!("Failed to stat Projection Fault Journal: {:?}", path))?
    {
        return Ok(ProjectionFaultJournal::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Projection Fault Journal: {:?}", path))?;
    let journal: ProjectionFaultJournal = toml::from_str(&content)
        .with_context(|| format!("Failed to parse Projection Fault Journal: {:?}", path))?;
    if journal.version != JOURNAL_VERSION {
        return Err(anyhow!(
            "Unsupported Projection Fault Journal version {} in {:?}",
            journal.version,
            path
        ));
    }
    Ok(journal)
}

fn write_journal_file(path: &Path, journal: &ProjectionFaultJournal) -> Result<()> {
    if journal.faults.is_empty() {
        remove_journal_file(path)?;
        return Ok(());
    }
    let Some(parent) = path.parent() else {
        return Err(anyhow!(
            "Projection Fault Journal path has no parent: {:?}",
            path
        ));
    };
    std::fs::create_dir_all(parent).with_context(|| {
        format!(
            "Failed to create Projection Fault Journal parent: {:?}",
            parent
        )
    })?;
    let content =
        toml::to_string_pretty(journal).context("Failed to serialize Projection Fault Journal")?;
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write Projection Fault Journal: {:?}", path))
}

fn remove_journal_file(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err)
            .with_context(|| format!("Failed to remove Projection Fault Journal: {:?}", path)),
    }
}

fn bounded_error(error: &str) -> String {
    if error.chars().count() <= MAX_ERROR_CHARS {
        return error.to_string();
    }
    let mut truncated = error.chars().take(MAX_ERROR_CHARS).collect::<String>();
    truncated.push_str("...");
    truncated
}

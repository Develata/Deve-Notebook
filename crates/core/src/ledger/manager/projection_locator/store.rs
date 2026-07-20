//! plan_ref:
//!   - 03_storage/projection#projection-locator-contract
//!
//! Durable store half of the Projection Locator runtime: the on-disk file
//! format, the cross-process map lock, and atomic read/replace I/O. Command
//! and query semantics stay in the parent module.

use super::{ProjectionLocatorRecord, file_validation};
use crate::models::RepoId;
use crate::utils::fs::{create_atomic_replace_temp, replace_file_atomically, sync_directory};
use crate::utils::notegit;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

const LOCATOR_VERSION: u32 = 2;
const LOCATOR_FILE: &str = "projection-locators.toml";
const LOCATOR_LOCK_FILE: &str = "projection-locators.lock";

/// Serializes every locator-map read-modify-write across threads and
/// processes. Mutators rewrite the whole map file, so an unguarded
/// prepare/set/remove pair from concurrent lifecycle jobs would silently drop
/// the other job's entry on atomic replace.
pub(super) struct ProjectionLocatorMapGuard {
    _file: std::fs::File,
}

impl ProjectionLocatorMapGuard {
    pub(super) fn acquire(ledger_dir: &Path) -> Result<Self> {
        let host_dir = notegit::host_dir(ledger_dir);
        std::fs::create_dir_all(&host_dir)?;
        let path = host_dir.join(LOCATOR_LOCK_FILE);
        let file = crate::utils::fs::open_regular_file_lock(&path, "projection locator map lock")?;
        file.lock()
            .context("Failed to lock projection locator map")?;
        crate::utils::fs::ensure_open_file_matches_path(
            &file,
            &path,
            "projection locator map lock",
        )?;
        Ok(Self { _file: file })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ProjectionLocatorFile {
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) locators: Vec<ProjectionLocatorRecord>,
}

impl Default for ProjectionLocatorFile {
    fn default() -> Self {
        Self {
            version: LOCATOR_VERSION,
            locators: Vec::new(),
        }
    }
}

pub(super) fn projection_locator_path_for(ledger_dir: &Path) -> PathBuf {
    notegit::host_dir(ledger_dir).join(LOCATOR_FILE)
}

pub(super) fn read_projection_locator_file(path: &Path) -> Result<ProjectionLocatorFile> {
    if !path
        .try_exists()
        .with_context(|| format!("Failed to stat Projection Locator file: {:?}", path))?
    {
        return Ok(ProjectionLocatorFile::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Projection Locator file: {:?}", path))?;
    let file: ProjectionLocatorFile = toml::from_str(&content)
        .with_context(|| format!("Failed to parse Projection Locator file: {:?}", path))?;
    if file.version != LOCATOR_VERSION {
        return Err(anyhow!(
            "Unsupported Projection Locator version {} in {:?}",
            file.version,
            path
        ));
    }
    file_validation::validate_projection_locator_file_shape(&file.locators)?;
    Ok(file)
}

pub(crate) fn projection_locator_record_for_repo_id(
    ledger_dir: &Path,
    repo_id: RepoId,
) -> Result<Option<ProjectionLocatorRecord>> {
    let file = read_projection_locator_file(&projection_locator_path_for(ledger_dir))?;
    let Some(mut record) = file
        .locators
        .into_iter()
        .find(|record| record.repo_id == repo_id)
    else {
        return Ok(None);
    };
    record.projection_base_abs =
        std::fs::canonicalize(&record.projection_base_abs).with_context(|| {
            format!(
                "Failed to canonicalize Projection Locator base for repo {}: {:?}",
                repo_id, record.projection_base_abs
            )
        })?;
    Ok(Some(record))
}

pub(super) fn write_projection_locator_file(
    path: &Path,
    file: &ProjectionLocatorFile,
) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Err(anyhow!("Projection Locator path has no parent: {:?}", path));
    };
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create Projection Locator parent: {:?}", parent))?;
    let content = toml::to_string_pretty(file).context("Failed to serialize Projection Locator")?;
    let temp = parent.join(format!(
        ".{LOCATOR_FILE}.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let write_result = (|| -> Result<()> {
        let mut handle = create_atomic_replace_temp(&temp)
            .with_context(|| format!("Failed to create Projection Locator temp: {:?}", temp))?;
        handle
            .write_all(content.as_bytes())
            .with_context(|| format!("Failed to write Projection Locator temp: {:?}", temp))?;
        handle
            .sync_all()
            .with_context(|| format!("Failed to sync Projection Locator temp: {:?}", temp))?;
        replace_file_atomically(&handle, &temp, path)
            .with_context(|| format!("Failed to replace Projection Locator file: {:?}", path))?;
        sync_directory(parent)
            .with_context(|| format!("Failed to sync Projection Locator parent: {:?}", parent))?;
        Ok(())
    })();
    match write_result {
        Ok(()) => Ok(()),
        Err(primary) => match std::fs::remove_file(&temp) {
            Ok(()) => Err(primary),
            Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => Err(primary),
            Err(cleanup) => Err(anyhow!("{primary}; temp cleanup also failed: {cleanup}")),
        },
    }
}

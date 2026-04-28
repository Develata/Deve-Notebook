//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!   - 04_storage#repo-runtime-layout
//!
use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};

use crate::ledger::manager::types::RepoManager;

impl RepoManager {
    pub(crate) fn checked_remotes_dir(&self) -> Result<PathBuf> {
        checked_existing_dir(
            self.remotes_dir(),
            "checking remote repo catalog",
            "remote repo directory",
            "Broken remote repo catalog",
        )
    }

    pub(crate) fn checked_local_dir_for(ledger_dir: &Path, context: &str) -> Result<PathBuf> {
        checked_existing_dir(
            ledger_dir.join("local"),
            context,
            "local repo directory",
            "Broken local repo catalog",
        )
    }
}

fn checked_existing_dir(
    path: PathBuf,
    context: &str,
    label: &str,
    broken_prefix: &str,
) -> Result<PathBuf> {
    match path.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            return Err(anyhow!("{broken_prefix}: {label} missing at {:?}", path));
        }
        Err(err) => {
            return Err(err)
                .with_context(|| format!("Failed to stat {label} while {context}: {:?}", path));
        }
    }
    let metadata = std::fs::metadata(&path).with_context(|| {
        format!(
            "Failed to read {label} metadata while {context}: {:?}",
            path
        )
    })?;
    if !metadata.is_dir() {
        return Err(anyhow!("{broken_prefix}: expected directory at {:?}", path));
    }
    Ok(path)
}

//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 06_backup#projection-backup-pull-state-machine-contract
//!
//! Conditional rollback and repair preservation for an applied pull batch.

use deve_core::remote_projection::RemoteProjectionProviderError;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufReader, ErrorKind, Read};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct AppliedPullFiles {
    staging: Option<PathBuf>,
    applied: Vec<AppliedPullFile>,
    created_dirs: Vec<PathBuf>,
    committed: bool,
    rollback_on_drop: bool,
}

impl AppliedPullFiles {
    pub(super) fn empty() -> Self {
        Self {
            staging: None,
            applied: Vec::new(),
            created_dirs: Vec::new(),
            committed: true,
            rollback_on_drop: false,
        }
    }

    pub(super) fn pending(
        staging: PathBuf,
        applied: Vec<AppliedPullFile>,
        created_dirs: Vec<PathBuf>,
    ) -> Self {
        Self {
            staging: Some(staging),
            applied,
            created_dirs,
            committed: false,
            rollback_on_drop: true,
        }
    }

    pub(crate) fn defer_rollback(mut self) -> Self {
        self.rollback_on_drop = false;
        self
    }

    pub(crate) fn commit(mut self) {
        self.committed = true;
        self.cleanup_staging();
    }

    pub(crate) fn rollback_after_failed_scan(
        mut self,
    ) -> Result<(), RemoteProjectionProviderError> {
        self.committed = true;
        let result = rollback_applied_pull_files(&mut self.applied, &mut self.created_dirs);
        self.cleanup_staging();
        result
    }

    pub(crate) fn rollback_after_failed_scan_if_unchanged(
        mut self,
    ) -> Result<(), RemoteProjectionProviderError> {
        ensure_applied_fingerprints(&self.applied)?;
        self.committed = true;
        let result = rollback_applied_pull_files(&mut self.applied, &mut self.created_dirs);
        self.cleanup_staging();
        result
    }

    fn cleanup_staging(&mut self) {
        if let Some(staging) = self.staging.take() {
            let _ = fs::remove_dir_all(staging);
        }
    }
}

impl Drop for AppliedPullFiles {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if !self.rollback_on_drop {
            tracing::error!(
                staging = ?self.staging,
                "remote projection pull finalizer dropped; preserving staging for repair"
            );
            return;
        }
        if let Err(err) = rollback_applied_pull_files(&mut self.applied, &mut self.created_dirs) {
            tracing::warn!("failed to roll back remote projection pull workspace apply: {err}");
        }
        self.cleanup_staging();
    }
}

#[derive(Debug)]
pub(super) struct AppliedPullFile {
    target: PathBuf,
    backup: Option<PathBuf>,
    applied_fingerprint: [u8; 32],
}

impl AppliedPullFile {
    pub(super) fn new(
        target: PathBuf,
        backup: Option<PathBuf>,
        applied_fingerprint: [u8; 32],
    ) -> Self {
        Self {
            target,
            backup,
            applied_fingerprint,
        }
    }
}

fn ensure_applied_fingerprints(
    applied: &[AppliedPullFile],
) -> Result<(), RemoteProjectionProviderError> {
    for item in applied {
        match fs::symlink_metadata(&item.target) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {}
            _ => {
                tracing::error!(target = %item.target.display(), "remote pull rollback target changed");
                return Err(rollback_conflict());
            }
        }
        if file_sha256(&item.target)? != item.applied_fingerprint {
            tracing::error!(target = %item.target.display(), "remote pull rollback fingerprint changed");
            return Err(rollback_conflict());
        }
    }
    Ok(())
}

pub(super) fn file_sha256(path: &Path) -> Result<[u8; 32], RemoteProjectionProviderError> {
    let file = fs::File::open(path).map_err(|error| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "failed to open projection fingerprint target: {error}"
        ))
    })?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer).map_err(|error| {
            RemoteProjectionProviderError::ProviderIo(format!(
                "failed to read projection fingerprint target: {error}"
            ))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

fn rollback_conflict() -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(
        "remote projection rollback blocked by a newer workspace change; repair required"
            .to_string(),
    )
}

pub(super) fn rollback_applied_pull_files(
    applied: &mut Vec<AppliedPullFile>,
    created_dirs: &mut Vec<PathBuf>,
) -> Result<(), RemoteProjectionProviderError> {
    let mut rollback_errors = Vec::new();
    while let Some(item) = applied.pop() {
        match fs::symlink_metadata(&item.target) {
            Ok(metadata) if metadata.is_file() || metadata.file_type().is_symlink() => {
                if let Err(err) = fs::remove_file(&item.target) {
                    rollback_errors.push(format!("remove {}: {err}", item.target.display()));
                }
            }
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => rollback_errors.push(format!("stat {}: {err}", item.target.display())),
        }
        if let Some(backup) = item.backup
            && let Err(err) = fs::rename(&backup, &item.target)
        {
            rollback_errors.push(format!(
                "restore {} from {}: {err}",
                item.target.display(),
                backup.display()
            ));
        }
    }
    while let Some(dir) = created_dirs.pop() {
        match fs::remove_dir(&dir) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => rollback_errors.push(format!("remove dir {}: {err}", dir.display())),
        }
    }
    if rollback_errors.is_empty() {
        Ok(())
    } else {
        Err(RemoteProjectionProviderError::ProviderIo(
            rollback_errors.join("; "),
        ))
    }
}

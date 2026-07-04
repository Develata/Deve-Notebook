//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use deve_core::remote_projection::{RemoteProjectionFile, RemoteProjectionProviderError};
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub(super) fn write_pull_files(
    workspace: &Path,
    files: &[RemoteProjectionFile],
) -> Result<(), RemoteProjectionProviderError> {
    if files.is_empty() {
        return Ok(());
    }
    let workspace_root = workspace.canonicalize().map_err(|err| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "failed to canonicalize projection workspace {}: {err}",
            workspace.display()
        ))
    })?;
    let targets = validate_pull_targets(&workspace_root, files)?;
    let staging = stage_pull_files(files)?;
    let result = apply_staged_pull_files(&workspace_root, &staging, &targets);
    let _ = fs::remove_dir_all(&staging);
    result
}

#[derive(Debug)]
struct PullTarget {
    relative_path: String,
    target: PathBuf,
}

#[derive(Debug)]
struct AppliedPullFile {
    target: PathBuf,
    backup: Option<PathBuf>,
}

fn validate_pull_targets(
    workspace_root: &Path,
    files: &[RemoteProjectionFile],
) -> Result<Vec<PullTarget>, RemoteProjectionProviderError> {
    let mut targets = Vec::with_capacity(files.len());
    for file in files {
        let relative = Path::new(file.path());
        validate_existing_parent_chain(workspace_root, relative)?;
        let target = workspace_root.join(relative);
        reject_existing_unsafe_target(&target)?;
        targets.push(PullTarget {
            relative_path: file.path().to_string(),
            target,
        });
    }
    Ok(targets)
}

fn stage_pull_files(
    files: &[RemoteProjectionFile],
) -> Result<PathBuf, RemoteProjectionProviderError> {
    let staging = std::env::temp_dir().join(format!("deve-webdav-pull-{}", Uuid::new_v4()));
    fs::create_dir(&staging).map_err(|err| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "failed to create WebDAV pull staging directory {}: {err}",
            staging.display()
        ))
    })?;
    for file in files {
        let staged = staging.join(file.path());
        let parent = staged.parent().ok_or_else(|| {
            RemoteProjectionProviderError::ProviderIo(format!(
                "projection file has no staging parent: {}",
                file.path()
            ))
        })?;
        if let Err(err) =
            fs::create_dir_all(parent).and_then(|_| fs::write(&staged, file.content()))
        {
            let _ = fs::remove_dir_all(&staging);
            return Err(RemoteProjectionProviderError::ProviderIo(format!(
                "failed to stage projection file {}: {err}",
                file.path()
            )));
        }
    }
    Ok(staging)
}

fn apply_staged_pull_files(
    workspace_root: &Path,
    staging: &Path,
    targets: &[PullTarget],
) -> Result<(), RemoteProjectionProviderError> {
    let backup_root = staging.join(format!("__backup-{}", Uuid::new_v4()));
    fs::create_dir(&backup_root).map_err(|err| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "failed to create WebDAV pull backup directory {}: {err}",
            backup_root.display()
        ))
    })?;

    let mut applied = Vec::new();
    for target in targets {
        if let Err(err) = create_safe_parent(workspace_root, &target.target)
            .and_then(|_| replace_from_staging(staging, &backup_root, target, &mut applied))
        {
            rollback_applied_pull_files(&mut applied, err)?;
        }
    }
    Ok(())
}

fn replace_from_staging(
    staging: &Path,
    backup_root: &Path,
    target: &PullTarget,
    applied: &mut Vec<AppliedPullFile>,
) -> Result<(), RemoteProjectionProviderError> {
    reject_existing_unsafe_target(&target.target)?;
    let staged = staging.join(&target.relative_path);
    let backup = if target.target.exists() {
        let backup = backup_root.join(&target.relative_path);
        let parent = backup.parent().ok_or_else(|| {
            RemoteProjectionProviderError::ProviderIo(format!(
                "projection backup has no parent: {}",
                target.relative_path
            ))
        })?;
        fs::create_dir_all(parent).map_err(|err| {
            RemoteProjectionProviderError::ProviderIo(format!(
                "failed to create projection backup directory {}: {err}",
                parent.display()
            ))
        })?;
        fs::rename(&target.target, &backup).map_err(|err| {
            RemoteProjectionProviderError::ProviderIo(format!(
                "failed to backup projection file {}: {err}",
                target.target.display()
            ))
        })?;
        Some(backup)
    } else {
        None
    };
    applied.push(AppliedPullFile {
        target: target.target.clone(),
        backup,
    });

    let temp_target = temporary_target_path(&target.target)?;
    if let Err(err) = fs::copy(&staged, &temp_target) {
        let _ = fs::remove_file(&temp_target);
        return Err(RemoteProjectionProviderError::ProviderIo(format!(
            "failed to copy staged projection file {}: {err}",
            target.relative_path
        )));
    }
    fs::rename(&temp_target, &target.target).map_err(|err| {
        let _ = fs::remove_file(&temp_target);
        RemoteProjectionProviderError::ProviderIo(format!(
            "failed to install projection file {}: {err}",
            target.target.display()
        ))
    })?;
    Ok(())
}

fn rollback_applied_pull_files(
    applied: &mut Vec<AppliedPullFile>,
    original: RemoteProjectionProviderError,
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
    if rollback_errors.is_empty() {
        Err(original)
    } else {
        Err(RemoteProjectionProviderError::ProviderIo(format!(
            "{original}; rollback failed: {}",
            rollback_errors.join("; ")
        )))
    }
}

fn validate_existing_parent_chain(
    workspace_root: &Path,
    relative: &Path,
) -> Result<(), RemoteProjectionProviderError> {
    let Some(parent) = relative.parent() else {
        return Err(RemoteProjectionProviderError::ProviderIo(format!(
            "projection file has no parent: {}",
            relative.display()
        )));
    };
    let mut current = workspace_root.to_path_buf();
    for component in parent.components() {
        let Component::Normal(segment) = component else {
            return Err(RemoteProjectionProviderError::InvalidProjectionPath);
        };
        current.push(segment);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(RemoteProjectionProviderError::ProviderIo(format!(
                    "projection parent escapes workspace through symlink: {}",
                    current.display()
                )));
            }
            Ok(metadata) if metadata.is_dir() => {
                let canonical = current.canonicalize().map_err(|err| {
                    RemoteProjectionProviderError::ProviderIo(format!(
                        "failed to canonicalize projection parent {}: {err}",
                        current.display()
                    ))
                })?;
                if !canonical.starts_with(workspace_root) {
                    return Err(RemoteProjectionProviderError::ProviderIo(format!(
                        "projection parent escapes workspace: {}",
                        current.display()
                    )));
                }
            }
            Ok(_) => {
                return Err(RemoteProjectionProviderError::ProviderIo(format!(
                    "projection parent is not a directory: {}",
                    current.display()
                )));
            }
            Err(err) if err.kind() == ErrorKind::NotFound => break,
            Err(err) => {
                return Err(RemoteProjectionProviderError::ProviderIo(format!(
                    "failed to inspect projection parent {}: {err}",
                    current.display()
                )));
            }
        }
    }
    Ok(())
}

fn create_safe_parent(
    workspace_root: &Path,
    target: &Path,
) -> Result<(), RemoteProjectionProviderError> {
    let parent = target.parent().ok_or_else(|| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "projection file has no parent: {}",
            target.display()
        ))
    })?;
    let relative_parent = parent.strip_prefix(workspace_root).map_err(|_| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "projection target escapes workspace: {}",
            target.display()
        ))
    })?;
    let mut current = workspace_root.to_path_buf();
    for component in relative_parent.components() {
        let Component::Normal(segment) = component else {
            return Err(RemoteProjectionProviderError::InvalidProjectionPath);
        };
        current.push(segment);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(RemoteProjectionProviderError::ProviderIo(format!(
                    "projection parent escapes workspace through symlink: {}",
                    current.display()
                )));
            }
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                return Err(RemoteProjectionProviderError::ProviderIo(format!(
                    "projection parent is not a directory: {}",
                    current.display()
                )));
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|err| {
                    RemoteProjectionProviderError::ProviderIo(format!(
                        "failed to create projection directory {}: {err}",
                        current.display()
                    ))
                })?;
            }
            Err(err) => {
                return Err(RemoteProjectionProviderError::ProviderIo(format!(
                    "failed to inspect projection parent {}: {err}",
                    current.display()
                )));
            }
        }
    }
    let canonical = parent.canonicalize().map_err(|err| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "failed to canonicalize projection parent {}: {err}",
            parent.display()
        ))
    })?;
    if canonical.starts_with(workspace_root) {
        Ok(())
    } else {
        Err(RemoteProjectionProviderError::ProviderIo(format!(
            "projection parent escapes workspace: {}",
            parent.display()
        )))
    }
}

fn reject_existing_unsafe_target(target: &Path) -> Result<(), RemoteProjectionProviderError> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(RemoteProjectionProviderError::ProviderIo(format!(
                "projection target escapes workspace through symlink: {}",
                target.display()
            )))
        }
        Ok(metadata) if metadata.is_dir() => Err(RemoteProjectionProviderError::ProviderIo(
            format!("projection target is a directory: {}", target.display()),
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(RemoteProjectionProviderError::ProviderIo(format!(
            "failed to inspect projection target {}: {err}",
            target.display()
        ))),
    }
}

fn temporary_target_path(target: &Path) -> Result<PathBuf, RemoteProjectionProviderError> {
    let parent = target.parent().ok_or_else(|| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "projection file has no parent: {}",
            target.display()
        ))
    })?;
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            RemoteProjectionProviderError::ProviderIo(format!(
                "projection file name is not UTF-8: {}",
                target.display()
            ))
        })?;
    Ok(parent.join(format!(".{file_name}.deve-pull-{}.tmp", Uuid::new_v4())))
}

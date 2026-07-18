//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 06_backup#remote-import-runtime-boundary
//!
//! Exact Ledger, locator and ignore-file baseline capture for Remote Import.

use super::super::error::{RemoteImportError, RemoteImportResult};
use super::super::types::{RemoteImportBaseline, RemoteImportDigest};
use crate::ledger::traits::{RepoSelector, Repository};
use crate::ledger::{RepoManager, range};
use crate::models::{GlobalSeq, RepoId};
use crate::utils::path::{path_to_forward_slash, to_forward_slash};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::Path;

const MAX_IGNORE_SNAPSHOT_BYTES: u64 = 2 * 1024 * 1024;

pub(super) fn capture_baseline(
    repo: &RepoManager,
    expected_repo_id: RepoId,
    repo_name: &str,
) -> RemoteImportResult<RemoteImportBaseline> {
    let info = repo
        .get_repo_info_for(None, Some(repo_name))
        .map_err(RemoteImportError::storage)?
        .ok_or_else(|| RemoteImportError::Storage("Remote Import repo is missing".to_string()))?;
    if info.uuid != expected_repo_id {
        return Err(RemoteImportError::ApplyFailed(
            "Remote Import repo membership changed".to_string(),
        ));
    }
    let execution_name = repo
        .resolve_local_repo_name_for_execution(Some(expected_repo_id), Some(repo_name))
        .map_err(RemoteImportError::storage)?;
    let head_before = repo
        .run_on_local_repo(&execution_name, range::get_max_seq)
        .map_err(RemoteImportError::storage)?;
    let selector = RepoSelector {
        repo_id: Some(expected_repo_id),
        repo_name: Some(execution_name.clone()),
    };
    let mut existing = BTreeMap::new();
    for (doc_id, path) in repo
        .list_docs_in_repo(&selector)
        .map_err(RemoteImportError::storage)?
    {
        let path = to_forward_slash(&path);
        let content = repo
            .get_doc_content_in_repo(&selector, doc_id)
            .map_err(RemoteImportError::storage)?;
        if existing
            .insert(path.clone(), RemoteImportDigest::of(content.as_bytes()))
            .is_some()
        {
            return Err(RemoteImportError::Storage(format!(
                "Remote Import authority contains duplicate path {path:?}"
            )));
        }
    }
    let head_after = repo
        .run_on_local_repo(&execution_name, range::get_max_seq)
        .map_err(RemoteImportError::storage)?;
    if head_before != head_after {
        return Err(RemoteImportError::ApplyFailed(
            "Remote Import authority changed while capturing baseline".to_string(),
        ));
    }
    let workspace_root = repo
        .local_repo_workspace_root(&execution_name)
        .map_err(RemoteImportError::storage)?;
    Ok(RemoteImportBaseline {
        ledger_head: GlobalSeq::from_storage_key(head_before),
        ignore_digest: ignore_snapshot_digest(&workspace_root)?,
        locator_digest: projection_locator_digest(repo, expected_repo_id, &execution_name)?,
        existing,
    })
}

pub(super) fn projection_locator_digest(
    repo: &RepoManager,
    expected_repo_id: RepoId,
    repo_name: &str,
) -> RemoteImportResult<RemoteImportDigest> {
    let info = repo
        .get_repo_info_for(None, Some(repo_name))
        .map_err(RemoteImportError::storage)?
        .ok_or_else(|| RemoteImportError::Storage("Remote Import repo is missing".to_string()))?;
    if info.uuid != expected_repo_id {
        return Err(RemoteImportError::ApplyFailed(
            "Remote Import repo membership changed".to_string(),
        ));
    }
    let locator = repo
        .validated_projection_locator_for_repo_id(expected_repo_id)
        .map_err(RemoteImportError::storage)?;
    let workspace_root = repo
        .local_repo_workspace_root(repo_name)
        .map_err(RemoteImportError::storage)?;
    let base = path_to_forward_slash(&locator.projection_base_abs);
    let root = path_to_forward_slash(&workspace_root);
    let mut material =
        Vec::with_capacity(base.len() + root.len() + locator.repo_name_hint.len() + 96);
    material.extend_from_slice(b"deve-remote-import-projection-locator-v1\0");
    material.extend_from_slice(expected_repo_id.as_bytes());
    append_digest_field(&mut material, locator.repo_name_hint.as_bytes());
    append_digest_field(&mut material, base.as_bytes());
    append_digest_field(&mut material, root.as_bytes());
    Ok(RemoteImportDigest::of(&material))
}

fn append_digest_field(material: &mut Vec<u8>, field: &[u8]) {
    material.extend_from_slice(&(field.len() as u64).to_le_bytes());
    material.extend_from_slice(field);
}

pub(super) fn ignore_snapshot_digest(
    workspace_root: &Path,
) -> RemoteImportResult<RemoteImportDigest> {
    let path = workspace_root.join(".deveignore");
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(RemoteImportDigest::of(
                b"deve-remote-import-ignore-v1\0absent",
            ));
        }
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(RemoteImportError::ArtifactTampered(
            ".deveignore must be a regular non-symlink file".to_string(),
        ));
    }
    if metadata.len() > MAX_IGNORE_SNAPSHOT_BYTES {
        return Err(RemoteImportError::LimitExceeded {
            kind: "ignore snapshot bytes",
            limit: MAX_IGNORE_SNAPSHOT_BYTES,
            observed: metadata.len(),
        });
    }
    let identity = file_id::get_file_id(&path).map_err(RemoteImportError::source_read)?;
    let mut file = File::open(&path)?;
    let opened = file.metadata()?;
    if opened.len() != metadata.len() || !opened.is_file() {
        return Err(RemoteImportError::SourceRead(
            ".deveignore changed while opening".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(opened.len() as usize);
    file.by_ref()
        .take(MAX_IGNORE_SNAPSHOT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let after = std::fs::symlink_metadata(&path)?;
    let after_identity = file_id::get_file_id(&path).map_err(RemoteImportError::source_read)?;
    if after_identity != identity
        || after.len() != opened.len()
        || bytes.len() as u64 > MAX_IGNORE_SNAPSHOT_BYTES
    {
        return Err(RemoteImportError::SourceRead(
            ".deveignore changed while capturing snapshot".to_string(),
        ));
    }
    let mut material = Vec::with_capacity(bytes.len() + 48);
    material.extend_from_slice(b"deve-remote-import-ignore-v1\0present\0");
    material.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    material.extend_from_slice(&bytes);
    Ok(RemoteImportDigest::of(&material))
}

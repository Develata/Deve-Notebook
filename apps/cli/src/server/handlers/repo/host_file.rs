//! plan_ref:
//!   - 03_storage/projection#projection-locator-contract
//!   - 03_storage/index#internal-path-normalization
//!   - 04_repository#repo-selector-resolution-contract
//!   - 11_ui_design/index#context-action-surface
//!
//! Host-file context actions for file tree targets.

use crate::server::{AppState, node_role};
use axum::extract::{Json, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::{ServerError, ServerErrorCode};
use deve_core::utils::notegit;
use deve_core::utils::path::{join_normalized, path_to_forward_slash, to_forward_slash};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct HostFileQuery {
    pub path: String,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

#[derive(Deserialize)]
pub struct HostFileRevealRequest {
    pub path: String,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

#[derive(Serialize)]
struct HostFilePathResponse {
    absolute_path: String,
}

pub async fn absolute_path(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HostFileQuery>,
) -> Response {
    match ensure_supported(HostFileAction::CopyAbsolutePath)
        .and_then(|_| resolve_host_file_target(&state, &query.repo, &query.path))
    {
        Ok(target) => Json(HostFilePathResponse {
            absolute_path: display_absolute_path(&target),
        })
        .into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn reveal(
    State(state): State<Arc<AppState>>,
    Json(request): Json<HostFileRevealRequest>,
) -> Response {
    match ensure_supported(HostFileAction::RevealInSystemExplorer)
        .and_then(|_| resolve_host_file_target(&state, &request.repo, &request.path))
        .and_then(|target| reveal_target(&target))
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => err.into_response(),
    }
}

#[derive(Clone, Copy)]
enum HostFileAction {
    CopyAbsolutePath,
    RevealInSystemExplorer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostFileErrorKind {
    BadRequest,
    NotFound,
    Conflict,
    Internal,
}

#[derive(Debug)]
struct HostFileError {
    kind: HostFileErrorKind,
    detail: String,
}

impl HostFileError {
    fn bad_request(detail: impl Into<String>) -> Self {
        Self {
            kind: HostFileErrorKind::BadRequest,
            detail: detail.into(),
        }
    }

    fn not_found(detail: impl Into<String>) -> Self {
        Self {
            kind: HostFileErrorKind::NotFound,
            detail: detail.into(),
        }
    }

    fn conflict(detail: impl Into<String>) -> Self {
        Self {
            kind: HostFileErrorKind::Conflict,
            detail: detail.into(),
        }
    }

    fn internal(detail: impl Into<String>) -> Self {
        Self {
            kind: HostFileErrorKind::Internal,
            detail: detail.into(),
        }
    }

    fn into_response(self) -> Response {
        let (status, code) = match self.kind {
            HostFileErrorKind::BadRequest => {
                (StatusCode::BAD_REQUEST, ServerErrorCode::RequestFailed)
            }
            HostFileErrorKind::NotFound => {
                (StatusCode::NOT_FOUND, ServerErrorCode::StorageNotFound)
            }
            HostFileErrorKind::Conflict => {
                (StatusCode::CONFLICT, ServerErrorCode::ScRepoContextInvalid)
            }
            HostFileErrorKind::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ServerErrorCode::StoragePersistFailed,
            ),
        };
        (status, Json(ServerError::with_detail(code, self.detail))).into_response()
    }
}

fn ensure_supported(action: HostFileAction) -> Result<(), HostFileError> {
    let role = node_role::get_node_role();
    let summary = node_role::host_file_actions_for(&role);
    let supported = match action {
        HostFileAction::CopyAbsolutePath => summary.copy_absolute_path,
        HostFileAction::RevealInSystemExplorer => summary.reveal_in_system_explorer,
    };
    if supported {
        Ok(())
    } else {
        Err(HostFileError::conflict(
            "Host file actions are not available for this node role",
        ))
    }
}

fn resolve_host_file_target(
    state: &Arc<AppState>,
    repo: &RepoSelector,
    raw_path: &str,
) -> Result<PathBuf, HostFileError> {
    let repo_name = state
        .repo
        .resolve_local_repo_name_for_execution(repo.repo_id, repo.repo_name.as_deref())
        .map_err(|err| HostFileError::conflict(err.to_string()))?;
    let rel_path = normalize_host_file_path(raw_path)?;
    let root = state
        .repo
        .validate_local_repo_workspace_identity(&repo_name)
        .map_err(|err| HostFileError::conflict(err.to_string()))?;
    canonical_repo_target(&root, &rel_path)
}

fn normalize_host_file_path(raw_path: &str) -> Result<String, HostFileError> {
    let path = to_forward_slash(raw_path)
        .trim()
        .trim_end_matches('/')
        .to_string();
    if path.is_empty() {
        return Err(HostFileError::bad_request(
            "host file path must not be empty",
        ));
    }
    if path.starts_with('/') || has_windows_drive_prefix(&path) {
        return Err(HostFileError::bad_request(
            "host file path must be repo-relative",
        ));
    }
    let mut segments = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(HostFileError::bad_request("host file path is invalid"));
        }
        if notegit::is_internal_repo_segment(segment) {
            return Err(HostFileError::bad_request(
                "host file path targets an internal repo directory",
            ));
        }
        segments.push(segment);
    }
    Ok(segments.join("/"))
}

fn canonical_repo_target(root: &Path, rel_path: &str) -> Result<PathBuf, HostFileError> {
    let root = std::fs::canonicalize(root).map_err(|err| {
        HostFileError::conflict(format!("Projection workspace unavailable: {err}"))
    })?;
    let target = join_normalized(&root, rel_path);
    let target = std::fs::canonicalize(&target).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            HostFileError::not_found(format!("host file target not found: {rel_path}"))
        } else {
            HostFileError::internal(format!("failed to canonicalize host file target: {err}"))
        }
    })?;
    if !target.starts_with(&root) {
        return Err(HostFileError::conflict(
            "host file target escapes Projection workspace",
        ));
    }
    Ok(target)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn reveal_target(target: &Path) -> Result<(), HostFileError> {
    let spec = reveal_command(target)?;
    Command::new(spec.program)
        .args(spec.args)
        .spawn()
        .map(|_| ())
        .map_err(|err| HostFileError::internal(format!("failed to reveal host file target: {err}")))
}

#[derive(Debug, PartialEq, Eq)]
struct RevealCommandSpec {
    program: &'static str,
    args: Vec<String>,
}

fn reveal_command(target: &Path) -> Result<RevealCommandSpec, HostFileError> {
    #[cfg(target_os = "windows")]
    {
        Ok(RevealCommandSpec {
            program: "explorer.exe",
            args: vec![format!("/select,{}", display_absolute_path(target))],
        })
    }
    #[cfg(target_os = "macos")]
    {
        Ok(RevealCommandSpec {
            program: "open",
            args: vec!["-R".into(), display_absolute_path(target)],
        })
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let open_target = if target.is_dir() {
            target
        } else {
            target.parent().ok_or_else(|| {
                HostFileError::conflict("host file target has no parent directory")
            })?
        };
        Ok(RevealCommandSpec {
            program: "xdg-open",
            args: vec![display_absolute_path(open_target)],
        })
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        Err(HostFileError::conflict(
            "system file manager reveal is unsupported on this platform",
        ))
    }
}

fn display_absolute_path(path: &Path) -> String {
    strip_windows_extended_prefix(&path_to_forward_slash(path))
        .replace('/', std::path::MAIN_SEPARATOR_STR)
}

fn strip_windows_extended_prefix(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("//?/UNC/") {
        return format!("//{rest}");
    }
    path.strip_prefix("//?/").unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_file_path_rejects_absolute_traversal_and_internal_segments() {
        for path in [
            "",
            "/notes/a.md",
            "C:/notes/a.md",
            "notes/../a.md",
            ".notegit/identity.toml",
            "notes/.git/config",
        ] {
            assert!(
                normalize_host_file_path(path).is_err(),
                "path must be rejected: {path}"
            );
        }
    }

    #[test]
    fn host_file_path_normalizes_windows_separators() {
        assert_eq!(
            normalize_host_file_path("notes\\a.md").expect("valid"),
            "notes/a.md"
        );
    }

    #[test]
    fn canonical_repo_target_requires_existing_target_inside_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let notes = dir.path().join("notes");
        std::fs::create_dir_all(&notes).expect("mkdir");
        std::fs::write(notes.join("a.md"), "a").expect("write");

        let target = canonical_repo_target(dir.path(), "notes/a.md").expect("target");
        assert!(target.ends_with("notes/a.md"));
        assert_eq!(
            canonical_repo_target(dir.path(), "missing.md")
                .expect_err("missing")
                .kind,
            HostFileErrorKind::NotFound
        );
    }

    #[cfg(unix)]
    #[test]
    fn canonical_repo_target_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("root");
        let outside = tempfile::tempdir().expect("outside");
        let outside_file = outside.path().join("secret.md");
        std::fs::write(&outside_file, "secret").expect("outside file");
        symlink(&outside_file, root.path().join("leak.md")).expect("symlink");

        assert_eq!(
            canonical_repo_target(root.path(), "leak.md")
                .expect_err("symlink escape")
                .kind,
            HostFileErrorKind::Conflict
        );
    }

    #[test]
    fn windows_extended_prefix_is_not_exposed_to_users() {
        assert_eq!(
            strip_windows_extended_prefix("//?/C:/notes/a.md"),
            "C:/notes/a.md"
        );
        assert_eq!(
            strip_windows_extended_prefix("//?/UNC/server/share/a.md"),
            "//server/share/a.md"
        );
    }
}

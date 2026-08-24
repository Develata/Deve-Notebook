//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!   - 10_rendering#document-authority-bridge

use super::errors;
use crate::server::channel::DualChannel;
use deve_core::protocol::doc_file_op_errors as path_err;
use deve_core::utils::path::to_forward_slash;

pub const MAX_DEPTH: usize = 10;

pub fn normalize_repo_path_input(path: &str) -> Option<String> {
    let path = to_forward_slash(path).trim().to_string();
    (!path.is_empty()).then_some(path)
}

pub fn validate_file_path(path: &str, ch: &DualChannel, scope_nonce: Option<u64>) -> bool {
    validate_path_with_channel(path, true, ch, scope_nonce)
}

pub fn validate_folder_path(path: &str, ch: &DualChannel, scope_nonce: Option<u64>) -> bool {
    validate_path_with_channel(path, false, ch, scope_nonce)
}

#[derive(Debug)]
pub(super) struct PathValidationError {
    pub detail: String,
}

pub(super) fn validate_create_file_path(path: &str) -> Result<(), PathValidationError> {
    validate_path_kind(path, true)
}

pub(super) fn validate_create_folder_path(path: &str) -> Result<(), PathValidationError> {
    validate_path_kind(path, false)
}

fn validate_path_with_channel(
    path: &str,
    allow_file_leaf: bool,
    ch: &DualChannel,
    scope_nonce: Option<u64>,
) -> bool {
    match validate_path_kind(path, allow_file_leaf) {
        Ok(()) => true,
        Err(error) => {
            errors::request_failed_scoped(ch, error.detail, scope_nonce);
            false
        }
    }
}

fn validate_path_kind(path: &str, allow_file_leaf: bool) -> Result<(), PathValidationError> {
    let Some(path) = normalize_repo_path_input(path) else {
        return Err(PathValidationError {
            detail: path_err::INVALID_EMPTY_PATH.to_string(),
        });
    };
    if path.contains("..") || path.starts_with('/') || has_windows_drive_prefix(&path) {
        tracing::error!("路径校验失败 (遍历攻击): {}", path);
        return Err(PathValidationError {
            detail: path_err::invalid_path(&path),
        });
    }
    let segments: Vec<_> = path
        .trim_end_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.len() > MAX_DEPTH {
        tracing::error!("路径校验失败 (深度超限): {}", path);
        return Err(PathValidationError {
            detail: path_err::depth_limit_exceeded(MAX_DEPTH),
        });
    }

    if segments.is_empty() {
        return Err(PathValidationError {
            detail: path_err::INVALID_EMPTY_PATH.to_string(),
        });
    }
    validate_segments(&path, allow_file_leaf, &segments)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn validate_segments(
    path: &str,
    allow_file_leaf: bool,
    segments: &[&str],
) -> Result<(), PathValidationError> {
    for (index, segment) in segments.iter().enumerate() {
        if deve_core::utils::notegit::is_internal_repo_segment(segment) {
            tracing::error!("路径校验失败 (保留目录): {}", path);
            return Err(PathValidationError {
                detail: path_err::reserved_internal_path(path),
            });
        }
        let is_leaf = index + 1 == segments.len();
        let md_dir = segment.ends_with(".md") && (!allow_file_leaf || !is_leaf);
        if md_dir {
            tracing::error!("路径校验失败 (.md 目录): {}", path);
            return Err(PathValidationError {
                detail: path_err::markdown_directory_forbidden(path),
            });
        }
    }
    Ok(())
}

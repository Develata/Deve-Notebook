//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!   - 10_rendering#document-authority-bridge

use super::errors;
use crate::server::channel::DualChannel;
use deve_core::protocol::doc_file_op_errors as path_err;

pub const MAX_DEPTH: usize = 10;

pub fn validate_file_path(path: &str, ch: &DualChannel, scope_nonce: Option<u64>) -> bool {
    validate_path_kind(path, true, ch, scope_nonce)
}

pub fn validate_folder_path(path: &str, ch: &DualChannel, scope_nonce: Option<u64>) -> bool {
    validate_path_kind(path, false, ch, scope_nonce)
}

fn validate_path_kind(
    path: &str,
    allow_file_leaf: bool,
    ch: &DualChannel,
    scope_nonce: Option<u64>,
) -> bool {
    if path.contains("..") || path.starts_with('/') || path.starts_with('\\') {
        tracing::error!("路径校验失败 (遍历攻击): {}", path);
        errors::request_failed_scoped(ch, path_err::invalid_path(path), scope_nonce);
        return false;
    }
    if std::path::Path::new(path).components().count() > MAX_DEPTH {
        tracing::error!("路径校验失败 (深度超限): {}", path);
        errors::request_failed_scoped(ch, path_err::depth_limit_exceeded(MAX_DEPTH), scope_nonce);
        return false;
    }

    let segments: Vec<_> = path
        .trim_end_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        errors::request_failed_scoped(ch, path_err::INVALID_EMPTY_PATH, scope_nonce);
        return false;
    }
    validate_segments(path, allow_file_leaf, &segments, ch, scope_nonce)
}

fn validate_segments(
    path: &str,
    allow_file_leaf: bool,
    segments: &[&str],
    ch: &DualChannel,
    scope_nonce: Option<u64>,
) -> bool {
    for (index, segment) in segments.iter().enumerate() {
        if deve_core::utils::notegit::is_internal_repo_segment(segment) {
            tracing::error!("路径校验失败 (保留目录): {}", path);
            errors::request_failed_scoped(ch, path_err::reserved_internal_path(path), scope_nonce);
            return false;
        }
        let is_leaf = index + 1 == segments.len();
        let md_dir = segment.ends_with(".md") && (!allow_file_leaf || !is_leaf);
        if md_dir {
            tracing::error!("路径校验失败 (.md 目录): {}", path);
            errors::request_failed_scoped(
                ch,
                path_err::markdown_directory_forbidden(path),
                scope_nonce,
            );
            return false;
        }
    }
    true
}

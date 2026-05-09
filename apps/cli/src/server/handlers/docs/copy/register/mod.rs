//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Copied document registration entry point.

use super::super::errors;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{ResolvedRepo, local_repo_root};
use std::path::Path;
use std::sync::Arc;

mod dirs;
mod files;
mod path;

#[derive(Clone, Copy)]
pub(super) struct CopyRegisterCtx<'a> {
    pub state: &'a Arc<AppState>,
    pub ch: &'a DualChannel,
    pub scope: &'a ResolvedRepo,
    pub scope_nonce: Option<u64>,
}

pub(super) fn register_copied_docs(
    ctx: CopyRegisterCtx<'_>,
    src: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    let base = match local_repo_root(ctx.state, ctx.scope) {
        Ok(path) => path,
        Err(err) => {
            errors::classified_failure_scoped(ctx.ch, err.to_string(), ctx.scope_nonce);
            return false;
        }
    };
    dirs::register_dirs(ctx, src, &base, src_path, dest_path)
        && files::register_files(ctx, src, &base, src_path, dest_path)
}

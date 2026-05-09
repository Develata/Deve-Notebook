//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! 会话级 repo 解析辅助。
//!
//! Invariants:
//! - 进入底层 DB/Tree 算子前，必须先拿到真实 `RepoId`。
//! - 本地写路径不得静默回退到进程默认主库。

mod lookup;
mod repo_scope_bootstrap;
mod repo_scope_cleanup;
mod repo_scope_counterpart;
mod repo_scope_error;
mod repo_scope_remote;
mod repo_scope_resolve;
mod repo_scope_selector;
mod repo_scope_sync;
mod repo_scope_sync_bootstrap;
mod repo_scope_workspace;

use crate::server::AppState;
use anyhow::Result;
use std::sync::Arc;

pub(crate) use self::repo_scope_cleanup::should_clear_stale_remote_scope;
pub use self::repo_scope_error::{
    RepoScopeFailure, map_repo_scope_error, map_repo_scope_error_ref, stale_remote_scope_detail,
};
pub use self::repo_scope_resolve::{
    ResolvedRepo, bootstrap_local_repo, resolve_session_repo, stale_unbound_remote_scope_detail,
};
pub use self::repo_scope_sync::resolve_session_repo_and_sync;
pub use self::repo_scope_sync_bootstrap::resolve_session_repo_or_bootstrap_local;
pub use self::repo_scope_workspace::{
    local_repo_path, local_repo_root, run_on_resolved_local_repo,
};

/// 将当前 resolved scope 收敛到本地可写仓库。
/// Invariants: 已处于本地分支时直接返回当前 scope；远端影子仓库只允许按 `RepoUUID -> URL` 收敛到本地仓库；无可写本地对应仓库时显式返回 `None`。
pub fn resolve_local_counterpart_repo(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
) -> Result<Option<ResolvedRepo>> {
    repo_scope_counterpart::resolve_local_counterpart_repo(state, scope)
}

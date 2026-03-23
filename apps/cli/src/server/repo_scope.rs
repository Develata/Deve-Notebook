//! 会话级 repo 解析辅助。
//!
//! Invariants:
//! - 进入底层 DB/Tree 算子前，必须先拿到真实 `RepoId`。
//! - 本地写路径不得静默回退到进程默认主库。

#[path = "repo_scope_lookup.rs"]
mod lookup;
#[path = "repo_scope_bootstrap.rs"]
mod repo_scope_bootstrap;
#[path = "repo_scope_cleanup.rs"]
mod repo_scope_cleanup;
#[path = "repo_scope_counterpart.rs"]
mod repo_scope_counterpart;
#[path = "repo_scope_error.rs"]
mod repo_scope_error;
#[path = "repo_scope_remote.rs"]
mod repo_scope_remote;
#[path = "repo_scope_resolve.rs"]
mod repo_scope_resolve;
#[path = "repo_scope_selector.rs"]
mod repo_scope_selector;
#[path = "repo_scope_workspace.rs"]
mod repo_scope_workspace;

use crate::server::AppState;
use anyhow::Result;
use std::sync::Arc;

pub(crate) use self::repo_scope_cleanup::should_clear_stale_remote_scope;
pub use self::repo_scope_error::{map_repo_scope_error, stale_remote_scope_detail};
pub use self::repo_scope_resolve::{
    ResolvedRepo, bootstrap_local_repo, resolve_session_repo, resolve_session_repo_and_sync,
    resolve_session_repo_or_bootstrap_local, stale_unbound_remote_scope_detail,
};
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

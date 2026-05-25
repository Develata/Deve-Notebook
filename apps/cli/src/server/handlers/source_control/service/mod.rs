//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime

mod read;
mod target;
mod write;

use crate::server::repo_scope::ResolvedRepo;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ServerError;

pub type ScResult<T> = std::result::Result<T, ServerError>;

/// Invariants:
/// - Source Control 查询可以作用于 local 或 remote scope；写路径仍由上游 local-only resolver 保证。
/// - 上游 resolver 已负责将会话中的漂移 UUID 收敛到单一 repo。
/// - 下游 RepoSelector 必须同时携带 `repo_id + repo_name`，让底层 resolver 做一致性校验。
pub fn selector_from_scope(scope: &ResolvedRepo) -> RepoSelector {
    RepoSelector {
        repo_id: Some(scope.repo_id),
        repo_name: Some(scope.repo_name.clone()),
    }
}

pub use read::*;
pub use target::*;
pub use write::*;

#[cfg(test)]
mod tests {
    use super::selector_from_scope;
    use crate::server::repo_scope::ResolvedRepo;

    #[test]
    fn selector_from_scope_keeps_uuid_and_name() {
        let repo_id = uuid::Uuid::new_v4();
        let selector = selector_from_scope(&ResolvedRepo {
            repo_id,
            repo_name: "test".into(),
            branch: None,
        });
        assert_eq!(selector.repo_id, Some(repo_id));
        assert_eq!(selector.repo_name.as_deref(), Some("test"));
    }
}

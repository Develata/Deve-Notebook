#[path = "service/read.rs"]
mod read;
#[path = "service/target.rs"]
mod target;
#[path = "service/write.rs"]
mod write;

use crate::server::repo_scope::ResolvedRepo;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ServerError;

pub type ScResult<T> = std::result::Result<T, ServerError>;

/// Invariants:
/// - Source Control 只在本地 branch 上执行。
/// - 本地工作区与 side table 最终以 repo_name 选中具体仓库文件。
/// - 上游 `resolve_current_local_repo` 已负责将会话中的漂移 UUID 收敛到正确 repo_name。
pub fn selector_from_scope(scope: &ResolvedRepo) -> RepoSelector {
    RepoSelector {
        repo_id: None,
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
    fn selector_from_scope_prefers_local_repo_name() {
        let selector = selector_from_scope(&ResolvedRepo {
            repo_id: uuid::Uuid::new_v4(),
            repo_name: "test".into(),
            branch: None,
        });
        assert_eq!(selector.repo_id, None);
        assert_eq!(selector.repo_name.as_deref(), Some("test"));
    }
}

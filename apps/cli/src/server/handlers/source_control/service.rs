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

pub fn selector_from_scope(scope: &ResolvedRepo) -> RepoSelector {
    RepoSelector {
        repo_id: Some(scope.repo_id),
        repo_name: Some(scope.repo_name.clone()),
    }
}

pub use read::*;
pub use target::*;
pub use write::*;

// apps/cli/src/server/handlers/listing/mod.rs
//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 07_network#server-ws-runtime
//!
//! # 列表查询处理器
//!
//! 处理各类列表查询请求: ListDocs, ListShadows, ListRepos

mod docs;
mod repos;
mod scope;
mod shadows;

pub use docs::handle_list_docs;
pub use repos::handle_list_repos;
pub use shadows::handle_list_shadows;

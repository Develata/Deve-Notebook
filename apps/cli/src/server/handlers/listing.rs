// apps/cli/src/server/handlers/listing.rs
//! # 列表查询处理器
//!
//! 处理各类列表查询请求: ListDocs, ListShadows, ListRepos

#[path = "listing_docs.rs"]
mod listing_docs;
#[path = "listing_repos.rs"]
mod listing_repos;
#[path = "listing_scope.rs"]
mod listing_scope;
#[path = "listing_shadows.rs"]
mod listing_shadows;

pub use listing_docs::handle_list_docs;
pub use listing_repos::handle_list_repos;
pub use listing_shadows::handle_list_shadows;

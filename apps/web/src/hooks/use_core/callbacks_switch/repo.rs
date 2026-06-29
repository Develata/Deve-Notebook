//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!

mod create;
mod manage;
mod switch;

#[cfg(test)]
mod tests;

pub(super) use create::build_create_repo_callback;
pub(super) use manage::{build_remove_repo_callback, build_rename_repo_callback};
pub(super) use switch::build_switch_repo_callback;

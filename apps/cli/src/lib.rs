//! plan_ref:
//!   - 11_ui_design/index#native-post-gate-common-contract
//!   - 11_ui_design/02_desktop#desktop-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!
//! Library entrypoints shared by native shells.

mod admin_api;
mod cli;
#[allow(dead_code)]
mod commands;
mod dispatch;
mod dump_support;
mod export_entries;
mod graph_projection;
#[cfg(test)]
mod main_test;
pub mod native_runtime;
mod repo_init;
pub mod server;

#[cfg(test)]
pub(crate) use cli::Args;
pub use cli::run_cli;
#[cfg(test)]
pub(crate) use cli::run_pre_config_command;
pub(crate) use cli::{Commands, ConfigAction, NgitAction, RepoAction, RepoProjectionAction};
pub(crate) use commands::backup::BackupAction;
pub(crate) use commands::projection_remote::ProjectionRemoteAction;
pub(crate) use commands::sc::ScAction;

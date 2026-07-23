//! plan_ref:
//!   - 11_ui_design/index#native-post-gate-common-contract
//!   - 11_ui_design/02_desktop#desktop-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!
//! Library entrypoints shared by native shells.

mod admin_api;
mod cli;
mod cli_exit;
#[allow(dead_code)]
mod commands;
mod dispatch;
mod dump_support;
mod export_entries;
mod graph_projection;
mod local_cli_proxy_contract;
#[cfg(test)]
mod main_test;
pub mod native_runtime;
mod remote_import_runtime;
mod remote_import_wire;
mod remote_projection_transport;
mod repo_init;
pub mod server;
#[cfg(test)]
pub(crate) mod test_support;
mod watcher_runtime;
mod workspace_identity_gate;

#[cfg(test)]
pub(crate) use cli::Args;
pub use cli::run_cli;
#[cfg(test)]
pub(crate) use cli::run_pre_config_command;
pub(crate) use cli::{
    Commands, ConfigAction, NgitAction, RepoAction, RepoAliasAction, RepoProjectionAction,
};
pub use cli_exit::process_exit_code;
pub(crate) use commands::projection_remote::ProjectionRemoteAction;
pub(crate) use commands::remote_import::{LocalCliAuthArgs, RemoteImportAction};
pub(crate) use commands::sc::ScAction;

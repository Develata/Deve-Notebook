//! plan_ref:
//!   - 11_ui_design/index#native-post-gate-common-contract
//!   - 11_ui_design/02_desktop#desktop-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!
//! Library entrypoints shared by native shells.

mod admin_api;
#[allow(dead_code)]
mod commands;
mod dump_support;
mod export_entries;
mod graph_projection;
pub mod native_runtime;
mod repo_init;
pub mod server;

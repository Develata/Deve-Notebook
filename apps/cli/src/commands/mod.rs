// apps\cli\src\commands
//! CLI 子命令模块
//! plan_ref:
//!   - 14_commands#cli-commands
//!
//! 包含所有 CLI 支持的子命令实现。
pub mod backup;
pub mod config;
pub mod dump;
pub mod export;
pub mod git;
#[cfg(test)]
mod git_import_smoke_support;
#[cfg(test)]
mod git_import_smoke_test;
mod git_output;
pub mod graph;
pub mod init;
pub mod live_proxy;
pub mod merge_conflict_fixture;
pub mod node_check;
pub mod recover;
pub mod repair;
mod repo_arg;
pub mod repo_projection;
pub mod sc;
pub mod sc_status;
pub mod scan;
pub mod seed;
pub mod serve;
pub mod verify_p2p;
pub mod watch;

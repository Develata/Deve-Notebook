//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 14_commands#cli-commands

use crate::server::{
    channel::DualChannel, handlers::source_control::handle_resolve_conflict, session::WsSession,
};
use deve_core::git_bridge::apply_import;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::protocol::{ScPathTarget, ServerErrorCode, ServerMessage};
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, ConflictResolution};
use tokio::sync::mpsc;

use super::source_control_git_import_test_support as support;
use support::{
    build_state, create_imported_conflict_fixture, git, init_git_repo, write_workspace_file,
};

mod resolution;

mod rename;

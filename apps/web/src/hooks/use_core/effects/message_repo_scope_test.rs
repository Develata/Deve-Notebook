use super::{
    WriteReadyScopeInput, accepts_edit_rejected_message, accepts_protocol_error_message,
    accepts_write_ready, matches_projection_message_scope, matches_repo_scope,
};
use crate::api::ConnectionStatus;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::state::init_signals;
use deve_core::models::PeerId;
use leptos::prelude::*;

#[path = "message_repo_scope_test_errors.rs"]
mod errors;
#[path = "message_repo_scope_test_scope.rs"]
mod scope;
#[path = "message_repo_scope_test_write.rs"]
mod write;

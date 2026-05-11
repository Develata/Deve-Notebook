use super::{
    WriteReadyScopeInput, accepts_edit_rejected_message, accepts_protocol_error_message,
    accepts_write_ready, matches_projection_message_scope, matches_repo_scope,
};
use crate::api::ConnectionStatus;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::state::init_signals;
use deve_core::models::PeerId;
use leptos::prelude::*;

mod errors;
mod scope;
mod write;

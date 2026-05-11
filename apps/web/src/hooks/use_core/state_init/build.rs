//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::ConnectionStatus;
use leptos::prelude::*;

use super::{
    docs::DocSignals, repo::RepoSignals, runtime::RuntimeSignals,
    source_control::SourceControlSignals,
};
mod assemble;
mod spectator;
use self::{assemble::assemble_core_signals_with_spectator, spectator::build_is_spectator};

pub(super) fn assemble_core_signals(
    connection_status: ReadSignal<ConnectionStatus>,
    docs: DocSignals,
    repo: RepoSignals,
    runtime: RuntimeSignals,
    source_control: SourceControlSignals,
) -> super::super::state::CoreSignals {
    let is_spectator = build_is_spectator(connection_status, repo);
    assemble_core_signals_with_spectator(docs, repo, runtime, source_control, is_spectator)
}

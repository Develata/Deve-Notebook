use crate::api::ConnectionStatus;
use leptos::prelude::*;

use super::{
    docs::DocSignals, repo::RepoSignals, runtime::RuntimeSignals,
    source_control::SourceControlSignals,
};
#[path = "build_assemble.rs"]
mod assemble;
#[path = "build_spectator.rs"]
mod build_spectator;
use self::{assemble::assemble_core_signals_with_spectator, build_spectator::build_is_spectator};

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

//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#write-readiness
//!
//! Thin-client projection of the bounded RepoList -> ProtocolError remove partial.

use crate::api::WsService;
use crate::hooks::use_core::scope_prefs::clear_scope_pref;
use crate::hooks::use_core::state::CoreSignals;
use crate::hooks::use_core::types::LoadPhase;
use crate::i18n::{Locale, t};
use crate::runtime::remove_scope_partial::{
    ProtocolStageDecision, RemoveScopePartialStage, RemoveScopePartialStageKey,
    RepoListStageDecision, RepoListStageInput, RepoSwitchedStageDecision, classify_protocol_error,
    classify_repo_list, classify_repo_switched, monotonic_now_ms,
};
use deve_core::models::RepoId;
use deve_core::protocol::{RepoListEntry, ServerErrorCode};
use leptos::prelude::{GetUntracked, Set};

use super::message_control_runtime_repo::{clear_repo_scoped_runtime, request_repo_list};

pub(super) fn capture_repo_list(
    request_id: Option<&str>,
    branch: Option<&str>,
    scope_nonce: Option<u64>,
    repos: &[String],
    repo_entries: &[RepoListEntry],
    ws: &WsService,
    signals: CoreSignals,
) -> bool {
    let existing = signals.remove_scope_partial_stage.get_untracked();
    if let Some(existing) = existing.as_ref() {
        retire_stage(ws, signals, existing);
        return true;
    }
    let Some(current_repo_id) = signals
        .current_repo_id
        .get_untracked()
        .and_then(|repo_id| repo_id.parse().ok())
    else {
        return false;
    };
    let pending_repo_switch = signals.pending_repo_switch.get_untracked();
    let decision = classify_repo_list(
        None,
        RepoListStageInput {
            request_id,
            branch,
            scope_nonce,
            current_scope_nonce: signals.current_scope_nonce.get_untracked(),
            current_repo_id,
            pending_branch_switch: signals.pending_branch_switch.get_untracked().is_some(),
            pending_repo_switch: pending_repo_switch.as_ref(),
            repo_entries,
        },
    );
    let RepoListStageDecision::Stage(kind) = decision else {
        return matches!(decision, RepoListStageDecision::Retire);
    };
    let Some(scope_nonce) = scope_nonce else {
        return false;
    };
    let stage = RemoveScopePartialStage::new(
        ws.connection_epoch.get_untracked(),
        kind,
        scope_nonce,
        current_repo_id,
        repos.to_vec(),
        repo_entries.to_vec(),
        monotonic_now_ms(),
    );
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals.set_handshake_scope_nonce.set(None);
    signals
        .set_remove_scope_partial_stage
        .set(Some(stage.clone()));
    schedule_timeout(stage.key(), ws.clone(), signals);
    true
}

pub(super) fn capture_protocol_error(
    code: ServerErrorCode,
    switch_nonce: Option<u64>,
    scope_nonce: Option<u64>,
    ws: &WsService,
    locale: Locale,
    signals: CoreSignals,
) -> bool {
    let stage = signals.remove_scope_partial_stage.get_untracked();
    let decision = classify_protocol_error(
        stage.as_ref(),
        ws.connection_epoch.get_untracked(),
        code,
        switch_nonce,
        scope_nonce,
        monotonic_now_ms(),
    );
    match decision {
        ProtocolStageDecision::NotPartial => false,
        ProtocolStageDecision::Commit => {
            commit_no_scope(
                ws,
                locale,
                signals,
                stage.expect("commit requires staged RepoList"),
            );
            true
        }
        ProtocolStageDecision::Retire => {
            if let Some(stage) = stage.as_ref() {
                retire_stage(ws, signals, stage);
            }
            true
        }
    }
}

pub(super) enum RepoSwitchedStageAdmission {
    Unstaged,
    Staged(RemoveScopePartialStage),
    Rejected,
}

pub(super) fn admit_repo_switched(
    name: &str,
    uuid: &str,
    switch_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) -> RepoSwitchedStageAdmission {
    let stage = signals.remove_scope_partial_stage.get_untracked();
    let decision = match uuid.parse::<RepoId>() {
        Ok(repo_id) => classify_repo_switched(
            stage.as_ref(),
            ws.connection_epoch.get_untracked(),
            repo_id,
            name,
            switch_nonce,
            monotonic_now_ms(),
        ),
        Err(_) if stage.is_some() => RepoSwitchedStageDecision::Retire,
        Err(_) => RepoSwitchedStageDecision::NotPartial,
    };
    match decision {
        RepoSwitchedStageDecision::NotPartial => RepoSwitchedStageAdmission::Unstaged,
        RepoSwitchedStageDecision::Commit => {
            let stage = stage.expect("commit requires staged RepoList");
            let pending_matches =
                signals
                    .pending_repo_switch
                    .get_untracked()
                    .is_some_and(|pending| {
                        pending.is_remove_current() && Some(pending.switch_nonce) == switch_nonce
                    });
            if pending_matches {
                RepoSwitchedStageAdmission::Staged(stage)
            } else {
                retire_stage(ws, signals, &stage);
                RepoSwitchedStageAdmission::Rejected
            }
        }
        RepoSwitchedStageDecision::Retire => {
            if let Some(stage) = stage.as_ref() {
                retire_stage(ws, signals, stage);
            }
            RepoSwitchedStageAdmission::Rejected
        }
    }
}

pub(super) fn settle_repo_switched(
    admission: RepoSwitchedStageAdmission,
    accepted: bool,
    ws: &WsService,
    signals: CoreSignals,
) {
    let RepoSwitchedStageAdmission::Staged(stage) = admission else {
        return;
    };
    if !accepted {
        retire_stage(ws, signals, &stage);
        return;
    }
    signals.set_repo_list.set(stage.repos);
    signals.set_repo_entries.set(stage.repo_entries);
    signals.set_remove_scope_partial_stage.set(None);
}

pub(super) fn retire_stale_or_expired(ws: &WsService, signals: CoreSignals) {
    retire_stale_or_expired_at(ws, signals, monotonic_now_ms());
}

fn retire_stale_or_expired_at(ws: &WsService, signals: CoreSignals, now_mono_ms: u64) {
    let Some(stage) = signals.remove_scope_partial_stage.get_untracked() else {
        return;
    };
    if stage.connection_epoch != ws.connection_epoch.get_untracked()
        || stage.is_expired_at(now_mono_ms)
    {
        retire_stage(ws, signals, &stage);
    }
}

fn commit_no_scope(
    ws: &WsService,
    locale: Locale,
    signals: CoreSignals,
    stage: RemoveScopePartialStage,
) {
    clear_scope_pref();
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals.set_handshake_scope_nonce.set(None);
    signals.set_active_branch.set(None);
    signals.set_current_repo.set(None);
    signals.set_current_repo_id.set(None);
    signals.set_current_doc.set(None);
    signals.set_docs.set(Vec::new());
    signals.set_tree_nodes.set(Vec::new());
    signals.set_remove_scope_partial_stage.set(None);
    clear_repo_scoped_runtime(signals);
    signals.set_current_scope_nonce.set(stage.scope_nonce);
    signals.set_repo_list.set(stage.repos);
    signals.set_repo_entries.set(stage.repo_entries);
    signals.set_explicit_repo_selection_required.set(true);
    signals.set_sync_banner.set(Some(
        t::server_error::message(locale, ServerErrorCode::ScRepoNotSelected).to_string(),
    ));
}

fn retire_stage(ws: &WsService, signals: CoreSignals, stage: &RemoveScopePartialStage) {
    clear_scope_pref();
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals.set_handshake_scope_nonce.set(None);
    signals.set_active_branch.set(None);
    signals.set_current_repo.set(None);
    signals.set_current_repo_id.set(None);
    signals.set_current_doc.set(None);
    signals.set_docs.set(Vec::new());
    signals.set_tree_nodes.set(Vec::new());
    signals.set_repo_list.set(Vec::new());
    signals.set_remove_scope_partial_stage.set(None);
    clear_repo_scoped_runtime(signals);
    signals.set_explicit_repo_selection_required.set(true);
    signals.set_load_state.set(LoadPhase::Resyncing);

    let current_epoch = ws.connection_epoch.get_untracked();
    if stage.connection_epoch == current_epoch {
        ws.request_reconnect_for_resync(current_epoch);
    } else {
        request_repo_list(ws, signals);
    }
}

#[cfg(target_arch = "wasm32")]
fn schedule_timeout(key: RemoveScopePartialStageKey, ws: WsService, signals: CoreSignals) {
    gloo_timers::callback::Timeout::new(
        crate::runtime::remove_scope_partial::REMOVE_SCOPE_PARTIAL_STAGE_TIMEOUT_MS as u32,
        move || {
            let current = signals.remove_scope_partial_stage.get_untracked();
            if current.as_ref().map(RemoveScopePartialStage::key) == Some(key) {
                retire_stale_or_expired(&ws, signals);
            }
        },
    )
    .forget();
}

#[cfg(not(target_arch = "wasm32"))]
fn schedule_timeout(_key: RemoveScopePartialStageKey, _ws: WsService, _signals: CoreSignals) {}

#[cfg(test)]
#[path = "message_remove_scope_tests.rs"]
mod tests;

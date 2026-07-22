//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::runtime::document::pending;
use crate::runtime::domain::PendingRepoSwitch;
use deve_core::models::{PeerId, RepoId};
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::effects_switch;
use super::super::state::CoreSignals;
use super::message_control_runtime::{refresh_after_branch_switch, refresh_after_repo_switch};
use super::message_scope::string_branch_matches_scope;

pub fn handle_branch_switched(
    peer_id: Option<String>,
    success: bool,
    switch_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) {
    if effects_switch::handle_branch_switched(
        peer_id,
        success,
        switch_nonce,
        effects_switch::BranchSwitchSignals {
            pending_branch_switch: signals.pending_branch_switch,
            set_pending_branch_switch: signals.set_pending_branch_switch,
            set_active_branch: signals.set_active_branch,
        },
    ) {
        refresh_after_branch_switch(switch_nonce, ws, signals);
    }
}

pub fn handle_repo_switched(
    branch: Option<String>,
    name: String,
    uuid: String,
    switch_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) {
    if !string_branch_matches_scope(
        &branch,
        signals.active_branch.get_untracked(),
        signals
            .pending_branch_switch
            .get_untracked()
            .map(|pending| pending.into_target()),
    ) {
        leptos::logging::warn!("忽略 RepoSwitched: branch 与当前 scope 不匹配");
        return;
    }
    let active_branch = signals.active_branch.get_untracked();
    let current_repo_uuid = signals.current_repo_id.get_untracked();
    let pending_repo_switch = signals.pending_repo_switch.get_untracked();
    let explicit_repo_selection = pending_repo_switch
        .as_ref()
        .is_some_and(PendingRepoSwitch::is_explicit_switch);
    let session_restore_rebind = session_restore_rebind_target(SessionRestoreScopeInput {
        message_branch: branch.as_deref(),
        active_branch: active_branch.as_ref(),
        branch_switch_pending: signals.pending_branch_switch.get_untracked().is_some(),
        returned_repo_uuid: &uuid,
        switch_nonce,
        current_repo_uuid: current_repo_uuid.as_deref(),
        current_scope_nonce: signals.current_scope_nonce.get_untracked(),
        pending_repo_switch: pending_repo_switch.as_ref(),
    });
    let outcome = effects_switch::handle_repo_switched(
        name,
        uuid,
        switch_nonce,
        crate::hooks::use_core::RepoSwitchSignals {
            current_repo: signals.current_repo,
            current_repo_id: signals.current_repo_id,
            pending_repo_switch: signals.pending_repo_switch,
            set_pending_repo_switch: signals.set_pending_repo_switch,
            current_scope_nonce: signals.current_scope_nonce,
            set_current_scope_nonce: signals.set_current_scope_nonce,
            set_current_repo: signals.set_current_repo,
            set_current_repo_id: signals.set_current_repo_id,
            set_current_doc: signals.set_current_doc,
        },
    );
    if outcome.accepted
        && let Some((repo_id, previous_scope_nonce, next_scope_nonce)) = session_restore_rebind
    {
        let mut rebound = 0;
        signals.set_pending_local_edits.update(|pending_edits| {
            rebound = pending::rebind_pending_scope(
                pending_edits,
                repo_id,
                previous_scope_nonce,
                next_scope_nonce,
            );
        });
        if rebound > 0 {
            leptos::logging::log!(
                "Rebound {rebound} pending edits after same-repo internal session restore"
            );
        }
    }
    if outcome.accepted && explicit_repo_selection {
        signals.set_explicit_repo_selection_required.set(false);
    }
    if outcome.should_refresh {
        refresh_after_repo_switch(ws, signals);
    }
}

struct SessionRestoreScopeInput<'a> {
    message_branch: Option<&'a str>,
    active_branch: Option<&'a PeerId>,
    branch_switch_pending: bool,
    returned_repo_uuid: &'a str,
    switch_nonce: Option<u64>,
    current_repo_uuid: Option<&'a str>,
    current_scope_nonce: u64,
    pending_repo_switch: Option<&'a PendingRepoSwitch>,
}

fn session_restore_rebind_target(
    input: SessionRestoreScopeInput<'_>,
) -> Option<(RepoId, u64, u64)> {
    let pending = input
        .pending_repo_switch
        .filter(|pending| pending.restores_session_scope())?;
    let next_scope_nonce = input
        .switch_nonce
        .filter(|nonce| *nonce == pending.switch_nonce && *nonce > input.current_scope_nonce)?;
    if input.message_branch.is_some()
        || input.active_branch.is_some()
        || input.branch_switch_pending
    {
        return None;
    }
    let current_repo_uuid = input
        .current_repo_uuid
        .filter(|uuid| *uuid == input.returned_repo_uuid)?;
    let repo_id = current_repo_uuid.parse::<RepoId>().ok()?;
    Some((repo_id, input.current_scope_nonce, next_scope_nonce))
}

#[cfg(test)]
mod tests {
    use super::{SessionRestoreScopeInput, handle_repo_switched, session_restore_rebind_target};
    use crate::api::{ConnectionStatus, WsService};
    use crate::hooks::use_core::state::init_signals;
    use crate::runtime::document::pending::{
        PendingLocalEditInput, PendingScope, pending_count_for_doc_in_scope, push_pending_edit,
    };
    use crate::runtime::domain::PendingRepoSwitch;
    use deve_core::models::{DocId, Op, PeerId, RepoId};
    use deve_core::protocol::ClientMessage;
    use leptos::prelude::{GetUntracked, Owner, Set, Update, signal};

    #[test]
    fn exact_local_session_restore_can_rebind_pending_scope() {
        let repo_id = RepoId::new_v4();
        let repo_uuid = repo_id.to_string();
        let pending = PendingRepoSwitch::restore_session("default", repo_id, 8);

        assert_eq!(
            session_restore_rebind_target(SessionRestoreScopeInput {
                message_branch: None,
                active_branch: None,
                branch_switch_pending: false,
                returned_repo_uuid: &repo_uuid,
                switch_nonce: Some(8),
                current_repo_uuid: Some(&repo_uuid),
                current_scope_nonce: 7,
                pending_repo_switch: Some(&pending),
            }),
            Some((repo_id, 7, 8))
        );
    }

    #[test]
    fn user_switch_or_nonlocal_scope_cannot_rebind_pending() {
        let repo_id = RepoId::new_v4();
        let repo_uuid = repo_id.to_string();
        let user_switch = PendingRepoSwitch::switch("default", repo_id, 8);
        let restore = PendingRepoSwitch::restore_session("default", repo_id, 8);
        let branch = PeerId::random();

        assert!(
            session_restore_rebind_target(SessionRestoreScopeInput {
                message_branch: None,
                active_branch: None,
                branch_switch_pending: false,
                returned_repo_uuid: &repo_uuid,
                switch_nonce: Some(8),
                current_repo_uuid: Some(&repo_uuid),
                current_scope_nonce: 7,
                pending_repo_switch: Some(&user_switch),
            })
            .is_none()
        );
        let branch_text = branch.to_string();
        assert!(
            session_restore_rebind_target(SessionRestoreScopeInput {
                message_branch: Some(&branch_text),
                active_branch: Some(&branch),
                branch_switch_pending: true,
                returned_repo_uuid: &repo_uuid,
                switch_nonce: Some(8),
                current_repo_uuid: Some(&repo_uuid),
                current_scope_nonce: 7,
                pending_repo_switch: Some(&restore),
            })
            .is_none()
        );
    }

    #[test]
    fn uuid_or_nonce_mismatch_cannot_rebind_pending() {
        let repo_id = RepoId::new_v4();
        let repo_uuid = repo_id.to_string();
        let other_uuid = RepoId::new_v4().to_string();
        let pending = PendingRepoSwitch::restore_session("default", repo_id, 8);

        for (returned_uuid, nonce) in [(&other_uuid, Some(8)), (&repo_uuid, Some(9))] {
            assert!(
                session_restore_rebind_target(SessionRestoreScopeInput {
                    message_branch: None,
                    active_branch: None,
                    branch_switch_pending: false,
                    returned_repo_uuid: returned_uuid,
                    switch_nonce: nonce,
                    current_repo_uuid: Some(&repo_uuid),
                    current_scope_nonce: 7,
                    pending_repo_switch: Some(&pending),
                })
                .is_none()
            );
        }
    }

    #[test]
    fn accepted_session_restore_rebinds_without_sending_before_fresh_write_ready() {
        let runtime = Owner::new();
        runtime.set();
        let signals = init_signals(signal(ConnectionStatus::Connected).0);
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let repo_id = RepoId::new_v4();
        let repo_uuid = repo_id.to_string();
        let doc_id = DocId::new();
        signals.set_current_repo.set(Some("default".to_string()));
        signals.set_current_repo_id.set(Some(repo_uuid.clone()));
        signals.set_current_scope_nonce.set(7);
        signals.set_current_doc.set(Some(doc_id));
        signals
            .set_pending_repo_switch
            .set(Some(PendingRepoSwitch::restore_session(
                "default", repo_id, 8,
            )));
        signals.set_pending_local_edits.update(|pending| {
            push_pending_edit(
                pending,
                PendingLocalEditInput {
                    repo_id,
                    doc_id,
                    scope_nonce: 7,
                    client_id: 11,
                    client_op_id: 1,
                    base_version: 0,
                    op: Op::Insert {
                        pos: 0,
                        content: "pending".into(),
                    },
                },
            );
        });

        handle_repo_switched(
            None,
            "default".to_string(),
            repo_uuid,
            Some(8),
            &ws,
            signals,
        );

        assert_eq!(signals.current_scope_nonce.get_untracked(), 8);
        assert_eq!(
            pending_count_for_doc_in_scope(
                &signals.pending_local_edits.get_untracked(),
                doc_id,
                PendingScope {
                    repo_id,
                    scope_nonce: 8,
                },
            ),
            1
        );
        assert!(
            ws.drain_sent_for_test()
                .iter()
                .all(|message| !matches!(message, ClientMessage::Edit { .. })),
            "scope restore must wait for fresh WriteReady before replaying pending edits"
        );
    }
}

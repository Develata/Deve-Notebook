//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use deve_core::models::PeerId;
use leptos::prelude::*;

type MobileSurfaceRuntimeScope = (Option<String>, u64, Option<PeerId>);

pub(super) fn collapse_surface_switcher_on_runtime_transition(
    current_repo_id: ReadSignal<Option<String>>,
    current_scope_nonce: ReadSignal<u64>,
    active_branch: ReadSignal<Option<PeerId>>,
    branch_switch_pending: Signal<bool>,
    repo_switch_pending: Signal<bool>,
    set_surface_switcher_open: WriteSignal<bool>,
) {
    let last_surface_scope = StoredValue::new(mobile_surface_runtime_scope(
        current_repo_id.get_untracked(),
        current_scope_nonce.get_untracked(),
        active_branch.get_untracked(),
    ));

    Effect::new(move |_| {
        let next_scope = mobile_surface_runtime_scope(
            current_repo_id.get(),
            current_scope_nonce.get(),
            active_branch.get(),
        );
        let previous_scope = last_surface_scope.get_value();
        let scope_changed = mobile_surface_runtime_scope_changed(&previous_scope, &next_scope);
        if scope_changed {
            last_surface_scope.set_value(next_scope);
        }
        if mobile_surface_runtime_transition_should_close(
            scope_changed,
            branch_switch_pending.get(),
            repo_switch_pending.get(),
        ) {
            set_surface_switcher_open.set(false);
        }
    });
}

fn mobile_surface_runtime_scope(
    repo_id: Option<String>,
    scope_nonce: u64,
    active_branch: Option<PeerId>,
) -> MobileSurfaceRuntimeScope {
    (repo_id, scope_nonce, active_branch)
}

fn mobile_surface_runtime_scope_changed(
    previous: &MobileSurfaceRuntimeScope,
    current: &MobileSurfaceRuntimeScope,
) -> bool {
    previous != current
}

fn mobile_surface_runtime_transition_should_close(
    scope_changed: bool,
    branch_switch_pending: bool,
    repo_switch_pending: bool,
) -> bool {
    scope_changed || branch_switch_pending || repo_switch_pending
}

#[cfg(test)]
mod tests {
    use super::{
        mobile_surface_runtime_scope, mobile_surface_runtime_scope_changed,
        mobile_surface_runtime_transition_should_close,
    };
    use deve_core::models::PeerId;

    #[test]
    fn mobile_surface_switcher_closes_for_runtime_scope_transitions() {
        let local = mobile_surface_runtime_scope(Some("repo-a".to_string()), 3, None);
        let shadow = mobile_surface_runtime_scope(
            Some("repo-a".to_string()),
            3,
            Some(PeerId::new("peer-a")),
        );
        let new_scope = mobile_surface_runtime_scope(Some("repo-a".to_string()), 4, None);
        let new_repo = mobile_surface_runtime_scope(Some("repo-b".to_string()), 3, None);

        assert!(mobile_surface_runtime_scope_changed(&local, &shadow));
        assert!(mobile_surface_runtime_scope_changed(&local, &new_scope));
        assert!(mobile_surface_runtime_scope_changed(&local, &new_repo));
        assert!(!mobile_surface_runtime_scope_changed(&local, &local));
    }

    #[test]
    fn mobile_surface_switcher_closes_for_pending_branch_or_repo_switch() {
        assert!(mobile_surface_runtime_transition_should_close(
            true, false, false
        ));
        assert!(mobile_surface_runtime_transition_should_close(
            false, true, false
        ));
        assert!(mobile_surface_runtime_transition_should_close(
            false, false, true
        ));
        assert!(!mobile_surface_runtime_transition_should_close(
            false, false, false
        ));
    }
}

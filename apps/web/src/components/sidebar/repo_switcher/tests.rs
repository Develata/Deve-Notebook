use super::logic::{
    RepoSwitcherRow, repo_switcher_action_button_marker, repo_switcher_after_item_click,
    repo_switcher_after_outside_click, repo_switcher_after_trigger_click,
    repo_switcher_backdrop_marker, repo_switcher_can_submit_create_repo,
    repo_switcher_can_submit_rename_repo, repo_switcher_create_button_marker,
    repo_switcher_create_input_marker, repo_switcher_item_marker, repo_switcher_menu_marker,
    repo_switcher_remove_button_marker, repo_switcher_rename_button_marker,
    repo_switcher_rename_input_marker, repo_switcher_row_is_active, repo_switcher_row_is_renaming,
    repo_switcher_rows, repo_switcher_should_reset_transient_state, repo_switcher_switch_target,
    repo_switcher_trigger_marker,
};
use deve_core::protocol::{RepoListEntry, RepoReadiness};

#[test]
fn repo_switcher_trigger_marker_is_stable() {
    assert_eq!(repo_switcher_trigger_marker(), "repo-switcher-trigger");
}

#[test]
fn repo_switcher_menu_marker_only_exists_when_open() {
    assert_eq!(repo_switcher_menu_marker(true), Some("visible"));
    assert_eq!(repo_switcher_menu_marker(false), None);
}

#[test]
fn repo_switcher_backdrop_marker_is_stable() {
    assert_eq!(repo_switcher_backdrop_marker(), "repo-switcher-outside");
}

#[test]
fn repo_switcher_item_marker_is_stable() {
    assert_eq!(repo_switcher_item_marker(), "repo-switcher-item");
}

#[test]
fn repo_switcher_action_markers_are_stable() {
    assert_eq!(
        repo_switcher_action_button_marker(),
        "repo-switcher-actions"
    );
    assert_eq!(repo_switcher_rename_button_marker(), "repo-switcher-rename");
    assert_eq!(repo_switcher_remove_button_marker(), "repo-switcher-remove");
    assert_eq!(
        repo_switcher_rename_input_marker(),
        "repo-switcher-rename-input"
    );
}

#[test]
fn repo_switcher_trigger_click_toggles_menu() {
    assert!(repo_switcher_after_trigger_click(false));
    assert!(!repo_switcher_after_trigger_click(true));
}

#[test]
fn repo_switcher_outside_click_closes_menu() {
    assert!(!repo_switcher_after_outside_click());
}

#[test]
fn repo_switcher_item_click_closes_menu() {
    assert!(!repo_switcher_after_item_click());
}

#[test]
fn repo_switcher_closed_projection_retires_all_transient_state() {
    assert!(repo_switcher_should_reset_transient_state(false));
    assert!(!repo_switcher_should_reset_transient_state(true));
}

#[test]
fn repo_switcher_create_markers_are_stable() {
    assert_eq!(repo_switcher_create_button_marker(), "repo-switcher-create");
    assert_eq!(
        repo_switcher_create_input_marker(),
        "repo-switcher-create-input"
    );
}

#[test]
fn repo_switcher_create_submit_requires_non_empty_name() {
    assert!(!repo_switcher_can_submit_create_repo(""));
    assert!(!repo_switcher_can_submit_create_repo("   "));
    assert!(repo_switcher_can_submit_create_repo("research"));
}

#[test]
fn repo_switcher_rename_submit_requires_non_empty_changed_name() {
    assert!(!repo_switcher_can_submit_rename_repo("default", ""));
    assert!(!repo_switcher_can_submit_rename_repo("default", "   "));
    assert!(!repo_switcher_can_submit_rename_repo("default", "default"));
    assert!(repo_switcher_can_submit_rename_repo("default", "research"));
}

#[test]
fn repo_switcher_rename_state_requires_row_repo_id() {
    let repo_id = uuid::Uuid::new_v4();

    assert!(repo_switcher_row_is_renaming(Some(repo_id), repo_id));
    assert!(!repo_switcher_row_is_renaming(None, repo_id));
    assert!(!repo_switcher_row_is_renaming(
        Some(uuid::Uuid::new_v4()),
        repo_id
    ));
}

#[test]
fn repo_switcher_rows_require_protocol_entries() {
    let repo_id = uuid::Uuid::new_v4();
    let rows = repo_switcher_rows(vec![RepoListEntry {
        repo_id,
        display_alias: "display".to_string(),
        alias_revision: 3,
        readiness: RepoReadiness::Mounted,
    }]);

    assert_eq!(
        rows,
        vec![RepoSwitcherRow {
            repo_id,
            name: "display".to_string(),
            alias_revision: 3,
        }]
    );
}

#[test]
fn repo_switcher_rows_do_not_invent_name_only_identity() {
    assert!(repo_switcher_rows(Vec::new()).is_empty());
}

#[test]
fn repo_switcher_active_state_requires_repo_id() {
    let repo_id = uuid::Uuid::new_v4();
    let row = RepoSwitcherRow {
        repo_id,
        name: "default".to_string(),
        alias_revision: 0,
    };

    assert!(repo_switcher_row_is_active(Some(repo_id.to_string()), &row));
    assert!(!repo_switcher_row_is_active(None, &row));
}

#[test]
fn repo_switcher_active_state_does_not_fallback_to_display_when_ids_differ() {
    let active_repo_id = uuid::Uuid::new_v4();
    let inactive_repo_id = uuid::Uuid::new_v4();
    let row = RepoSwitcherRow {
        repo_id: inactive_repo_id,
        name: "shared".to_string(),
        alias_revision: 0,
    };

    assert!(!repo_switcher_row_is_active(
        Some(active_repo_id.to_string()),
        &row
    ));
}

#[test]
fn repo_switcher_switch_target_is_exact_repo_id() {
    let repo_id = uuid::Uuid::new_v4();
    let row = RepoSwitcherRow {
        repo_id,
        name: "display".to_string(),
        alias_revision: 4,
    };

    let target = repo_switcher_switch_target(&row);

    assert_eq!(target.expected_name, "display");
    assert_eq!(target.repo_id, repo_id);
}

#[test]
fn removal_dialog_is_a_single_typed_backend_projection() {
    let source = include_str!("removal_dialog.rs");

    assert!(source.contains("data-deve-repo-removal-dialog=\"visible\""));
    assert!(source.contains("data-deve-repo-removal-confirm=\"true\""));
    assert!(source.contains("role=\"dialog\""));
    assert!(source.contains("aria-modal=\"true\""));
    assert!(source.contains("min-h-[44px]"));
    assert!(source.contains("safe-area-inset-bottom"));
    assert!(source.contains("attach_modal_focus_restore_effect_with_fallback"));
    assert!(source.contains("preview.deleted"));
    assert!(source.contains("preview.preserved"));
    assert!(source.contains("preview.warnings"));
    assert!(source.contains("preview.blockers"));
    assert!(source.contains("value.can_execute"));
    assert!(!source.contains("window.confirm"));
    assert!(!source.contains("confirmation_token"));
    assert!(!source.contains("manifest"));
    assert!(!source.contains("detail"));
}

#[test]
fn remove_row_only_requests_backend_preview() {
    let source = include_str!("row/actions.rs");

    assert!(source.contains("on_remove_repo.run"));
    assert!(source.contains("min-h-[44px]"));
    assert!(!source.contains("ExecuteLocalRepoRemoval"));
    assert!(!source.contains("window.confirm"));
}

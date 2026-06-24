use super::logic::{
    RepoSwitcherRow, repo_switcher_action_button_marker, repo_switcher_after_item_click,
    repo_switcher_after_outside_click, repo_switcher_after_trigger_click,
    repo_switcher_backdrop_marker, repo_switcher_can_submit_create_repo,
    repo_switcher_can_submit_rename_repo, repo_switcher_create_button_marker,
    repo_switcher_create_input_marker, repo_switcher_item_marker, repo_switcher_menu_marker,
    repo_switcher_remove_button_marker, repo_switcher_rename_button_marker,
    repo_switcher_rename_input_marker, repo_switcher_row_is_active, repo_switcher_rows,
    repo_switcher_trigger_marker,
};
use deve_core::protocol::RepoListEntry;

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
fn repo_switcher_rows_prefer_protocol_entries() {
    let repo_id = uuid::Uuid::new_v4();
    let rows = repo_switcher_rows(
        vec!["legacy".to_string()],
        vec![RepoListEntry {
            repo_id,
            name: "display".to_string(),
            execution_name: "display--id".to_string(),
        }],
    );

    assert_eq!(
        rows,
        vec![RepoSwitcherRow {
            repo_id: Some(repo_id),
            name: "display".to_string(),
            execution_name: "display--id".to_string(),
        }]
    );
}

#[test]
fn repo_switcher_rows_keep_legacy_names_without_entries() {
    let rows = repo_switcher_rows(vec!["default".to_string()], Vec::new());

    assert_eq!(
        rows,
        vec![RepoSwitcherRow {
            repo_id: None,
            name: "default".to_string(),
            execution_name: "default".to_string(),
        }]
    );
}

#[test]
fn repo_switcher_active_state_accepts_repo_id_or_display_name() {
    let repo_id = uuid::Uuid::new_v4();
    let row = RepoSwitcherRow {
        repo_id: Some(repo_id),
        name: "default".to_string(),
        execution_name: "default--id".to_string(),
    };

    assert!(repo_switcher_row_is_active(
        Some("other".to_string()),
        Some(repo_id.to_string()),
        &row
    ));
    assert!(repo_switcher_row_is_active(
        Some("default".to_string()),
        None,
        &row
    ));
    assert!(!repo_switcher_row_is_active(
        Some("other".to_string()),
        None,
        &row
    ));
}

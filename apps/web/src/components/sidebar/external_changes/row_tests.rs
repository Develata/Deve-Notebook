use super::{directory_name, external_change_is_overlap_blocked, external_change_key, file_name};
use deve_core::source_control::{ChangeDomain, ChangeEntry, ChangeStatus};

fn entry(path: &str, has_conflict: bool, domain: ChangeDomain) -> ChangeEntry {
    ChangeEntry {
        path: path.to_string(),
        renamed_from: None,
        doc_id: None,
        status: ChangeStatus::Modified,
        has_conflict,
        domain,
        base_seq: None,
        target_seq: None,
    }
}

#[test]
fn overlap_state_comes_from_backend_conflict_flag() {
    let blocked = entry("notes/a.md", true, ChangeDomain::WorkingDirectory);
    let clean = entry("notes/a.md", false, ChangeDomain::WorkingDirectory);

    assert!(external_change_is_overlap_blocked(&blocked));
    assert!(!external_change_is_overlap_blocked(&clean));
    assert_ne!(external_change_key(&blocked), external_change_key(&clean));
}

#[test]
fn path_display_splits_name_and_directory() {
    assert_eq!(file_name("notes/a.md"), "a.md");
    assert_eq!(directory_name("notes/a.md"), "notes");
    assert_eq!(file_name("a.md"), "a.md");
    assert_eq!(directory_name("a.md"), "");
}

#[test]
fn mobile_external_changes_touch_targets_min_size_bound() {
    let row_source = include_str!("row.rs");
    let view_source = include_str!("../external_changes.rs");

    assert!(row_source.contains("data-deve-mobile-touch-target=\"external-changes-action\""));
    assert!(
        view_source.contains("data-deve-mobile-touch-target=\"external-changes-section-header\"")
    );
    assert!(row_source.contains("class=\"inline-flex h-11 w-11"));
    assert!(row_source.contains("md:h-7 md:w-7"));
    assert!(row_source.contains("class=\"group flex min-h-11"));
    assert!(row_source.contains("md:min-h-9"));
}

#[test]
fn external_changes_row_default_click_opens_diff_without_button_bubbling() {
    let source = include_str!("row.rs");

    assert!(source.contains("on_get_doc_diff.run(entry_store.get_value())"));
    assert!(source.contains("event.stop_propagation();"));
}

#[test]
fn external_changes_keeps_its_action_surface_separate() {
    let source = include_str!("row.rs");

    assert!(!source.contains(concat!("change_item_", "action_surface")));
    assert!(!source.contains(concat!("ChangeItem", "ActionSurface")));
    assert!(!source.contains(concat!("change_item_", "conflict_actions")));
}

use super::{
    activity_more_after_item_click, activity_more_after_pin_click, activity_more_item_marker,
    activity_more_pin_action_marker, toggle_activity_more_pin,
};
use crate::components::activity_bar::SidebarView;

#[test]
fn activity_more_menu_items_cover_sidebar_entries() {
    assert_eq!(
        activity_more_item_marker(SidebarView::Explorer),
        "activity_more_item_explorer"
    );
    assert_eq!(
        activity_more_item_marker(SidebarView::Search),
        "activity_more_item_search"
    );
    assert_eq!(
        activity_more_item_marker(SidebarView::SourceControl),
        "activity_more_item_source_control"
    );
    assert_eq!(
        activity_more_item_marker(SidebarView::Extensions),
        "activity_more_item_extensions"
    );
}

#[test]
fn activity_more_pin_actions_cover_sidebar_entries() {
    assert_eq!(
        activity_more_pin_action_marker(SidebarView::Explorer),
        "activity_more_pin_explorer"
    );
    assert_eq!(
        activity_more_pin_action_marker(SidebarView::Search),
        "activity_more_pin_search"
    );
    assert_eq!(
        activity_more_pin_action_marker(SidebarView::SourceControl),
        "activity_more_pin_source_control"
    );
    assert_eq!(
        activity_more_pin_action_marker(SidebarView::Extensions),
        "activity_more_pin_extensions"
    );
}

#[test]
fn activity_more_item_click_closes_menu() {
    assert!(!activity_more_after_item_click());
}

#[test]
fn activity_more_pin_click_keeps_menu_state() {
    assert!(activity_more_after_pin_click(true));
    assert!(!activity_more_after_pin_click(false));
}

#[test]
fn activity_more_pin_toggle_adds_unpinned_view() {
    let mut pinned = vec![SidebarView::Explorer];

    assert!(toggle_activity_more_pin(&mut pinned, SidebarView::Search));
    assert_eq!(pinned, vec![SidebarView::Explorer, SidebarView::Search]);
}

#[test]
fn activity_more_pin_toggle_removes_pinned_view() {
    let mut pinned = vec![SidebarView::Explorer, SidebarView::Search];

    assert!(toggle_activity_more_pin(&mut pinned, SidebarView::Search));
    assert_eq!(pinned, vec![SidebarView::Explorer]);
}

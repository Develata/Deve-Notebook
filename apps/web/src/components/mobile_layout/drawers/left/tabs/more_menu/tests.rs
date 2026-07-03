use super::{
    mobile_more_after_escape, mobile_more_after_item_click, mobile_more_after_pin_click,
    mobile_more_item_button_class, mobile_more_item_marker, mobile_more_item_row_class,
    mobile_more_item_touch_target, mobile_more_menu_marker, mobile_more_pin_action_marker,
    mobile_more_pin_button_class, mobile_more_pin_label, mobile_more_pin_touch_target,
    more_item_class, toggle_mobile_more_pin,
};
use crate::components::activity_bar::SidebarView;
use crate::i18n::{Locale, t};

#[test]
fn mobile_sidebar_more_menu_marker_is_visible_when_open() {
    assert_eq!(mobile_more_menu_marker(true), Some("visible"));
    assert_eq!(mobile_more_menu_marker(false), None);
}

#[test]
fn mobile_sidebar_more_menu_items_cover_sidebar_entries() {
    assert_eq!(
        mobile_more_item_marker(SidebarView::Explorer),
        "more_menu_item_explorer"
    );
    assert_eq!(
        mobile_more_item_marker(SidebarView::Search),
        "more_menu_item_search"
    );
    assert_eq!(
        mobile_more_item_marker(SidebarView::SourceControl),
        "more_menu_item_source_control"
    );
    assert_eq!(
        mobile_more_item_marker(SidebarView::ExternalChanges),
        "more_menu_item_external_changes"
    );
    assert_eq!(
        mobile_more_item_marker(SidebarView::Extensions),
        "more_menu_item_extensions"
    );
}

#[test]
fn mobile_sidebar_more_pin_actions_cover_sidebar_entries() {
    assert_eq!(
        mobile_more_pin_action_marker(SidebarView::Explorer),
        "more_menu_pin_explorer"
    );
    assert_eq!(
        mobile_more_pin_action_marker(SidebarView::Search),
        "more_menu_pin_search"
    );
    assert_eq!(
        mobile_more_pin_action_marker(SidebarView::SourceControl),
        "more_menu_pin_source_control"
    );
    assert_eq!(
        mobile_more_pin_action_marker(SidebarView::ExternalChanges),
        "more_menu_pin_external_changes"
    );
    assert_eq!(
        mobile_more_pin_action_marker(SidebarView::Extensions),
        "more_menu_pin_extensions"
    );
}

#[test]
fn mobile_sidebar_more_menu_reuses_desktop_entry_classes() {
    for view in SidebarView::all() {
        assert_eq!(more_item_class(view), mobile_more_item_marker(view));
    }
}

#[test]
fn mobile_sidebar_more_mobile_touch_targets_are_at_least_44px() {
    let row_class = mobile_more_item_row_class(SidebarView::Explorer);
    let item_button_class = mobile_more_item_button_class();
    let pin_button_class = mobile_more_pin_button_class(true);

    assert!(row_class.contains("mobile-more-item"));
    assert!(row_class.contains("h-11"));
    assert!(!row_class.contains("px-3"));
    assert!(item_button_class.contains("flex-1"));
    assert!(item_button_class.contains("h-full"));
    assert!(item_button_class.contains("px-3"));
    assert!(pin_button_class.contains("h-full"));
    assert!(pin_button_class.contains("min-w-[44px]"));
    assert!(pin_button_class.contains("px-3"));
}

#[test]
fn mobile_sidebar_more_mobile_touch_targets_markers_are_stable() {
    assert_eq!(mobile_more_item_touch_target(), "mobile_more_items");
    assert_eq!(mobile_more_pin_touch_target(), "mobile_more_pin_actions");
}

#[test]
fn mobile_sidebar_more_item_click_closes_menu() {
    assert!(!mobile_more_after_item_click());
}

#[test]
fn mobile_sidebar_more_escape_closes_menu() {
    assert!(!mobile_more_after_escape());
}

#[test]
fn mobile_sidebar_more_pin_click_keeps_menu_state() {
    assert!(mobile_more_after_pin_click(true));
    assert!(!mobile_more_after_pin_click(false));
}

#[test]
fn mobile_sidebar_more_pin_label_is_localized_by_state() {
    assert_eq!(
        mobile_more_pin_label(false, Locale::Zh),
        t::common::pin(Locale::Zh)
    );
    assert_eq!(
        mobile_more_pin_label(true, Locale::Zh),
        t::common::unpin(Locale::Zh)
    );
}

#[test]
fn mobile_sidebar_more_pin_toggle_adds_unpinned_view() {
    let mut pinned = vec![SidebarView::Explorer];

    assert!(toggle_mobile_more_pin(&mut pinned, SidebarView::Search));
    assert_eq!(pinned, vec![SidebarView::Explorer, SidebarView::Search]);
}

#[test]
fn mobile_sidebar_more_pin_toggle_removes_pinned_view_when_not_last() {
    let mut pinned = vec![SidebarView::Explorer, SidebarView::Search];

    assert!(toggle_mobile_more_pin(&mut pinned, SidebarView::Search));
    assert_eq!(pinned, vec![SidebarView::Explorer]);
}

#[test]
fn mobile_sidebar_more_pin_toggle_keeps_last_view_pinned() {
    let mut pinned = vec![SidebarView::Explorer];

    assert!(!toggle_mobile_more_pin(&mut pinned, SidebarView::Explorer));
    assert_eq!(pinned, vec![SidebarView::Explorer]);
}

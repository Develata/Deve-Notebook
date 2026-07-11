use super::{FocusTrapTarget, resolve_tab_target};
use super::{should_restore_previous_focus, should_trap_tab_key};

#[test]
fn focus_scope_tab_from_last_wraps_to_first() {
    assert_eq!(
        resolve_tab_target(Some(2), 3, false),
        FocusTrapTarget::First
    );
}

#[test]
fn focus_scope_shift_tab_from_first_wraps_to_last() {
    assert_eq!(resolve_tab_target(Some(0), 3, true), FocusTrapTarget::Last);
}

#[test]
fn focus_scope_tab_inside_bounds_uses_native_order() {
    assert_eq!(
        resolve_tab_target(Some(1), 3, false),
        FocusTrapTarget::Native
    );
    assert_eq!(
        resolve_tab_target(Some(1), 3, true),
        FocusTrapTarget::Native
    );
}

#[test]
fn focus_scope_outside_modal_enters_trap() {
    assert_eq!(resolve_tab_target(None, 3, false), FocusTrapTarget::First);
    assert_eq!(resolve_tab_target(None, 3, true), FocusTrapTarget::Last);
}

#[test]
fn focus_scope_traps_only_plain_tab() {
    assert!(should_trap_tab_key("Tab", false, false, false));
    assert!(!should_trap_tab_key("Tab", true, false, false));
    assert!(!should_trap_tab_key("Tab", false, true, false));
    assert!(!should_trap_tab_key("Tab", false, false, true));
    assert!(!should_trap_tab_key("Enter", false, false, false));
}

#[test]
fn focus_scope_blurs_only_when_active_element_is_inside_surface() {
    assert!(super::should_blur_active_element_for_hidden_surface(true));
    assert!(!super::should_blur_active_element_for_hidden_surface(false));
}

#[test]
fn focus_restore_respects_the_active_modal_owner() {
    assert!(should_restore_previous_focus(false, false));
    assert!(should_restore_previous_focus(true, true));
    assert!(!should_restore_previous_focus(true, false));
}

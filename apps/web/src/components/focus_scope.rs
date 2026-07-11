//! plan_ref:
//!   - 11_ui_design/index#layout-navigation-and-focus
//!
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement, KeyboardEvent};

const FOCUSABLE_SELECTOR: &str = concat!(
    "a[href],",
    "button:not([disabled]),",
    "input:not([disabled]),",
    "select:not([disabled]),",
    "textarea:not([disabled]),",
    "[tabindex]:not([tabindex=\"-1\"])"
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FocusTrapTarget {
    Native,
    First,
    Last,
}

pub(crate) fn resolve_tab_target(
    active_index: Option<usize>,
    focusable_count: usize,
    shift: bool,
) -> FocusTrapTarget {
    if focusable_count == 0 {
        return FocusTrapTarget::Native;
    }
    match (active_index, shift) {
        (None, true) => FocusTrapTarget::Last,
        (None, false) => FocusTrapTarget::First,
        (Some(0), true) => FocusTrapTarget::Last,
        (Some(index), false) if index + 1 >= focusable_count => FocusTrapTarget::First,
        _ => FocusTrapTarget::Native,
    }
}

pub(crate) fn should_trap_tab_key(key: &str, ctrl: bool, meta: bool, alt: bool) -> bool {
    key == "Tab" && !ctrl && !meta && !alt
}

pub(crate) fn active_element() -> Option<Element> {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.active_element())
}

pub(crate) fn should_restore_previous_focus(
    active_modal_present: bool,
    previous_inside_active_modal: bool,
) -> bool {
    !active_modal_present || previous_inside_active_modal
}

fn active_modal_elements() -> Vec<Element> {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return Vec::new();
    };
    let Ok(nodes) = document.query_selector_all("[role=\"dialog\"][aria-modal=\"true\"]") else {
        return Vec::new();
    };
    (0..nodes.length())
        .filter_map(|index| nodes.get(index))
        .filter_map(|node| node.dyn_into::<Element>().ok())
        .collect()
}

pub(crate) fn should_blur_active_element_for_hidden_surface(active_inside_surface: bool) -> bool {
    active_inside_surface
}

pub(crate) fn blur_active_element_inside(root: &Element) -> bool {
    let Some(active) = active_element() else {
        return false;
    };
    if !should_blur_active_element_for_hidden_surface(element_contains(root, &active)) {
        return false;
    }
    let Ok(active) = active.dyn_into::<HtmlElement>() else {
        return false;
    };
    let _ = active.blur();
    true
}

pub(crate) fn focus_input_next_frame(input_ref: NodeRef<leptos::html::Input>) {
    request_animation_frame(move || {
        if let Some(input) = input_ref.get_untracked() {
            let _ = input.focus();
        }
    });
}

pub(crate) fn focus_button_next_frame(button_ref: NodeRef<leptos::html::Button>) {
    request_animation_frame(move || {
        if let Some(button) = button_ref.get_untracked() {
            let _ = button.focus();
        }
    });
}

pub(crate) fn attach_modal_focus_restore_effect(
    is_open: impl Fn() -> bool + Copy + 'static,
    initial_focus_ref: NodeRef<leptos::html::Button>,
) {
    let last_open = StoredValue::new_local(false);
    let previous_focus = StoredValue::new_local(None::<web_sys::Element>);

    Effect::new(move |_| {
        let open = is_open();
        let was_open = last_open.get_value();
        last_open.set_value(open);

        if open && !was_open {
            previous_focus.set_value(active_element());
            focus_button_next_frame(initial_focus_ref);
        } else if !open && was_open {
            let previous = previous_focus.get_value();
            previous_focus.set_value(None);
            restore_focus_next_frame(previous);
        }
    });
}

pub(crate) fn restore_focus_next_frame(previous: Option<Element>) {
    request_animation_frame(move || {
        request_animation_frame(move || {
            let active_modals = active_modal_elements();
            let active_modal_present = !active_modals.is_empty();
            let previous_inside_active_modal = previous.as_ref().is_some_and(|previous| {
                active_modals
                    .iter()
                    .any(|modal| element_contains(modal, previous))
            });
            if !should_restore_previous_focus(active_modal_present, previous_inside_active_modal) {
                return;
            }
            if let Some(previous) = previous
                && let Ok(previous) = previous.dyn_into::<HtmlElement>()
                && focus_element(&previous)
                && active_html_element_is(&previous)
            {
                return;
            }
            if !active_modal_present {
                focus_editor();
            }
        });
    });
}

pub(crate) fn handle_focus_trap_keydown(
    ev: &KeyboardEvent,
    panel_ref: NodeRef<leptos::html::Div>,
) -> bool {
    if !should_trap_tab_key(&ev.key(), ev.ctrl_key(), ev.meta_key(), ev.alt_key()) {
        return false;
    }
    let Some(panel) = panel_ref.get_untracked() else {
        return false;
    };
    let focusable = focusable_elements(&panel);
    let active_index = active_element().as_ref().and_then(|active| {
        focusable
            .iter()
            .position(|candidate| same_element(candidate, active))
    });

    match resolve_tab_target(active_index, focusable.len(), ev.shift_key()) {
        FocusTrapTarget::Native => false,
        FocusTrapTarget::First => {
            ev.prevent_default();
            focusable.first().map(focus_element).is_some()
        }
        FocusTrapTarget::Last => {
            ev.prevent_default();
            focusable.last().map(focus_element).is_some()
        }
    }
}

fn focus_editor() {
    if let Some(editor) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.query_selector(".cm-content").ok().flatten())
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    {
        let _ = editor.focus();
    }
}

fn focusable_elements(root: &HtmlElement) -> Vec<HtmlElement> {
    let Ok(nodes) = root.query_selector_all(FOCUSABLE_SELECTOR) else {
        return Vec::new();
    };
    (0..nodes.length())
        .filter_map(|idx| nodes.get(idx))
        .filter_map(|node| node.dyn_into::<HtmlElement>().ok())
        .filter(|element| !element.has_attribute("disabled") && element.tab_index() >= 0)
        .collect()
}

fn focus_element(element: &HtmlElement) -> bool {
    element.focus().is_ok()
}

fn same_element(left: &HtmlElement, right: &Element) -> bool {
    let left: &web_sys::Node = left.as_ref();
    let right: &web_sys::Node = right.as_ref();
    left.is_same_node(Some(right))
}

fn element_contains(root: &Element, element: &Element) -> bool {
    let root: &web_sys::Node = root.as_ref();
    let element: &web_sys::Node = element.as_ref();
    root.contains(Some(element))
}

fn active_html_element_is(element: &HtmlElement) -> bool {
    active_element().is_some_and(|active| same_element(element, &active))
}

#[cfg(test)]
mod tests {
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
}

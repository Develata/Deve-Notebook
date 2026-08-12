//! plan_ref:
//!   - 11_ui_design/index#overlay-back-coordination
//!
//! Shared presentation-only transitions for dismissing overlay projections.

use leptos::prelude::{Set, WriteSignal};
use web_sys::MouseEvent;

pub(crate) fn close_from_control(event: MouseEvent, set_open: WriteSignal<bool>) {
    close_in_event_order(|| event.stop_propagation(), || set_open.set(false));
}

pub(crate) fn run_action_from_control(event: MouseEvent, action: impl FnOnce()) {
    close_in_event_order(|| event.stop_propagation(), action);
}

fn close_in_event_order(stop_propagation: impl FnOnce(), close_projection: impl FnOnce()) {
    stop_propagation();
    close_projection();
}

#[cfg(test)]
mod tests {
    use super::close_in_event_order;
    use std::cell::RefCell;
    use std::rc::Rc;

    const SEARCH_HEADER: &str = include_str!("search_box/ui_sections.rs");
    const COMMAND_PALETTE: &str = include_str!("command_palette/ui.rs");

    #[test]
    fn explicit_overlay_close_controls_use_shared_transition() {
        assert!(SEARCH_HEADER.contains("overlay_lifecycle::close_from_control"));
        assert!(COMMAND_PALETTE.contains("overlay_lifecycle::close_from_control"));
        assert!(COMMAND_PALETTE.contains("overlay_lifecycle::run_action_from_control"));
    }

    #[test]
    fn close_transition_stops_delegated_propagation_before_unmount() {
        let steps = Rc::new(RefCell::new(Vec::new()));
        let stopped = steps.clone();
        let closed = steps.clone();

        close_in_event_order(
            move || stopped.borrow_mut().push("stop-propagation"),
            move || closed.borrow_mut().push("close-projection"),
        );

        assert_eq!(
            steps.borrow().as_slice(),
            ["stop-propagation", "close-projection"]
        );
    }
}

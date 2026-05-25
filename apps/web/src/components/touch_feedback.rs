//! plan_ref:
//!   - 11_ui_design_03_mobile#mobile-interaction-design
//!
//! Shared touch feedback classes for list-like mobile rows.

pub fn interactive_item_state_class(selected: bool, selectable: bool) -> &'static str {
    match (selected, selectable) {
        (true, true) => "bg-accent-subtle text-accent hover:bg-hover active:bg-active",
        (false, true) => "text-primary hover:bg-hover active:bg-active",
        (_, false) => "text-muted cursor-default",
    }
}

#[cfg(test)]
mod tests {
    use super::interactive_item_state_class;

    #[test]
    fn interactive_rows_share_hover_active_and_selected_semantics() {
        assert_eq!(
            interactive_item_state_class(true, true),
            "bg-accent-subtle text-accent hover:bg-hover active:bg-active"
        );
        assert_eq!(
            interactive_item_state_class(false, true),
            "text-primary hover:bg-hover active:bg-active"
        );
    }

    #[test]
    fn disabled_rows_do_not_advertise_interaction_feedback() {
        assert_eq!(
            interactive_item_state_class(false, false),
            "text-muted cursor-default"
        );
        assert_eq!(
            interactive_item_state_class(true, false),
            "text-muted cursor-default"
        );
    }
}

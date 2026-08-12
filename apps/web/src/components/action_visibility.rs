//! plan_ref:
//!   - 11_ui_design/index#action-visibility-contract
//!
//! Shared projection policy for discoverable row and header actions.

pub(crate) fn persistent_action_visibility_class() -> &'static str {
    "opacity-100"
}

pub(crate) fn persistent_action_button_class() -> &'static str {
    "deve-persistent-action inline-flex items-center justify-center rounded hover:bg-hover text-secondary transition-colors"
}

#[cfg(test)]
mod tests {
    use super::{persistent_action_button_class, persistent_action_visibility_class};

    #[test]
    fn primary_actions_are_not_hover_gated_on_any_port() {
        let class = persistent_action_visibility_class();
        assert!(class.contains("opacity-100"));
        assert!(!class.contains("opacity-0"));
        assert!(!class.contains("group-hover"));
    }

    #[test]
    fn persistent_actions_keep_mobile_touch_size_and_desktop_density() {
        let class = persistent_action_button_class();
        let css = include_str!("../../style/_widgets.css");

        assert!(class.contains("deve-persistent-action"));
        assert!(css.contains(".deve-persistent-action"));
        assert!(css.contains("height: 2.75rem"));
        assert!(css.contains("width: 2.75rem"));
        assert!(css.contains("@media (hover: hover) and (pointer: fine)"));
        assert!(!class.contains("md:"));
    }
}

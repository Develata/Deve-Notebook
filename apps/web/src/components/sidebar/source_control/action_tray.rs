//! plan_ref:
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!
//! Shared Source Control action tray classes.

pub const CHANGE_ITEM_ACTION_TRAY_CLASS: &str = "sc-action-tray flex items-center gap-0.5 mr-1";
pub const SECTION_ACTION_TRAY_CLASS: &str = "sc-action-tray flex items-center gap-1 text-primary";

#[cfg(test)]
mod tests {
    use super::{CHANGE_ITEM_ACTION_TRAY_CLASS, SECTION_ACTION_TRAY_CLASS};

    #[test]
    fn source_control_action_tray_uses_stable_css_gate() {
        for class in [CHANGE_ITEM_ACTION_TRAY_CLASS, SECTION_ACTION_TRAY_CLASS] {
            assert!(class.contains("sc-action-tray"));
            assert!(!class.contains("md:hidden"));
            assert!(!class.contains("md:group-hover"));
        }
    }
}

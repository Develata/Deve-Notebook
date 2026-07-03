//! plan_ref:
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!
//! Resource group visibility rules for the Source Control primary flow.

pub(super) fn should_render_resource_group(count: usize, show_empty_group: bool) -> bool {
    count > 0 || show_empty_group
}

#[cfg(test)]
mod tests {
    use super::should_render_resource_group;

    #[test]
    fn non_empty_source_control_state_keeps_all_primary_groups_visible() {
        assert!(should_render_resource_group(0, true));
        assert!(should_render_resource_group(1, false));
        assert!(!should_render_resource_group(0, false));
    }
}

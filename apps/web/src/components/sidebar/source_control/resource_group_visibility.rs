//! plan_ref:
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!
//! Resource group visibility rules for the Source Control primary flow.

pub(super) fn has_any_resource_group_changes(
    staged_count: usize,
    unstaged_count: usize,
    confirmed_count: usize,
) -> bool {
    staged_count > 0 || unstaged_count > 0 || confirmed_count > 0
}

pub(super) fn should_render_resource_group(count: usize, show_empty_group: bool) -> bool {
    count > 0 || show_empty_group
}

pub(super) fn section_bulk_action_disabled(count: usize, bulk_busy: bool, can_write: bool) -> bool {
    count == 0 || bulk_busy || !can_write
}

#[cfg(test)]
mod tests {
    use super::{
        has_any_resource_group_changes, section_bulk_action_disabled, should_render_resource_group,
    };

    #[test]
    fn non_empty_source_control_state_keeps_all_primary_groups_visible() {
        assert!(!has_any_resource_group_changes(0, 0, 0));
        assert!(has_any_resource_group_changes(0, 0, 1));
        assert!(has_any_resource_group_changes(0, 1, 0));
        assert!(has_any_resource_group_changes(1, 0, 0));

        assert!(should_render_resource_group(0, true));
        assert!(should_render_resource_group(1, false));
        assert!(!should_render_resource_group(0, false));
    }

    #[test]
    fn empty_resource_group_bulk_actions_are_disabled() {
        assert!(section_bulk_action_disabled(0, false, true));
        assert!(section_bulk_action_disabled(1, true, true));
        assert!(section_bulk_action_disabled(1, false, false));
        assert!(!section_bulk_action_disabled(1, false, true));
    }
}

//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent

pub(super) fn can_open_change_item_diff(
    has_repo: bool,
    pending_branch_switch: bool,
    pending_repo_switch: bool,
    read_blocked: bool,
) -> bool {
    has_repo && !pending_branch_switch && !pending_repo_switch && !read_blocked
}

#[cfg(test)]
mod tests {
    use super::can_open_change_item_diff;

    #[test]
    fn readonly_write_block_does_not_block_diff_reads() {
        assert!(can_open_change_item_diff(true, false, false, false));
    }

    #[test]
    fn diff_reads_fail_closed_without_scope_or_during_switch() {
        assert!(!can_open_change_item_diff(false, false, false, false));
        assert!(!can_open_change_item_diff(true, true, false, false));
        assert!(!can_open_change_item_diff(true, false, true, false));
        assert!(!can_open_change_item_diff(true, false, false, true));
    }
}

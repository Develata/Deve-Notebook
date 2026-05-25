//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
use deve_core::source_control::CommitInfo;

pub fn should_reset_compare_state(
    expanded: bool,
    has_repo: bool,
    has_file_diff: bool,
    has_deleted_no_doc_id_notice: bool,
    selected_commit_id: Option<&str>,
    compare_base_commit_id: Option<&str>,
    commit_history: &[CommitInfo],
) -> bool {
    !expanded
        || !has_repo
        || has_file_diff
        || has_deleted_no_doc_id_notice
        || has_missing_selected_commit(selected_commit_id, commit_history)
        || has_missing_selected_commit(compare_base_commit_id, commit_history)
}

pub fn has_compare_state_to_clear(
    selected_commit_id: Option<&str>,
    compare_base_commit_id: Option<&str>,
    commit_diff_request_id: Option<&str>,
    commit_diff_count: usize,
    has_notice: bool,
) -> bool {
    selected_commit_id.is_some()
        || compare_base_commit_id.is_some()
        || commit_diff_request_id.is_some()
        || commit_diff_count > 0
        || has_notice
}

fn has_missing_selected_commit(
    selected_commit_id: Option<&str>,
    commit_history: &[CommitInfo],
) -> bool {
    let Some(selected_commit_id) = selected_commit_id else {
        return false;
    };
    !commit_history
        .iter()
        .any(|commit| commit.id == selected_commit_id)
}

#[cfg(test)]
mod tests {
    use super::should_reset_compare_state;
    use deve_core::source_control::CommitInfo;

    fn commit(id: &str) -> CommitInfo {
        CommitInfo {
            id: id.to_string(),
            parent_id: None,
            message: format!("commit-{id}"),
            timestamp: 0,
            doc_count: 1,
            ledger_seq: 1,
        }
    }

    #[test]
    fn deleted_docless_notice_forces_compare_reset() {
        assert!(should_reset_compare_state(
            true,
            true,
            false,
            true,
            None,
            None,
            &[commit("aaa1111")]
        ));
        assert!(!should_reset_compare_state(
            true,
            true,
            false,
            false,
            None,
            None,
            &[commit("aaa1111")]
        ));
    }

    #[test]
    fn missing_selected_commit_forces_compare_reset() {
        assert!(should_reset_compare_state(
            true,
            true,
            false,
            false,
            Some("missing"),
            None,
            &[commit("aaa1111")]
        ));
    }

    #[test]
    fn missing_compare_base_forces_compare_reset() {
        assert!(should_reset_compare_state(
            true,
            true,
            false,
            false,
            None,
            Some("missing"),
            &[commit("aaa1111")]
        ));
    }

    #[test]
    fn file_diff_forces_compare_reset() {
        assert!(should_reset_compare_state(
            true,
            true,
            true,
            false,
            Some("aaa1111"),
            Some("bbb2222"),
            &[commit("aaa1111"), commit("bbb2222")]
        ));
    }

    #[test]
    fn only_resets_when_there_is_compare_state_to_clear() {
        assert!(!super::has_compare_state_to_clear(
            None, None, None, 0, false
        ));
        assert!(super::has_compare_state_to_clear(
            Some("aaa1111"),
            None,
            None,
            0,
            false
        ));
        assert!(super::has_compare_state_to_clear(
            None,
            None,
            Some("req-1"),
            0,
            false
        ));
        assert!(super::has_compare_state_to_clear(
            None, None, None, 1, false
        ));
        assert!(super::has_compare_state_to_clear(None, None, None, 0, true));
    }
}

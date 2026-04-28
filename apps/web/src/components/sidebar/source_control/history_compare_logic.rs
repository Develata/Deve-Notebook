//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 03_rendering#document-authority-bridge
//!
use deve_core::source_control::CommitInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistorySelectionAction {
    ToggleClosed,
    ClearBaseSelection,
    ShowParentDiff {
        parent_id: Option<String>,
        target_id: String,
    },
    ShowRangeDiff {
        base_id: String,
        target_id: String,
    },
}

pub fn resolve_history_selection(
    selected_commit_id: Option<&str>,
    compare_base_commit_id: Option<&str>,
    commit: &CommitInfo,
) -> HistorySelectionAction {
    if selected_commit_id == Some(commit.id.as_str()) {
        return HistorySelectionAction::ToggleClosed;
    }

    if let Some(base_id) = compare_base_commit_id {
        if base_id == commit.id {
            return HistorySelectionAction::ClearBaseSelection;
        }

        return HistorySelectionAction::ShowRangeDiff {
            base_id: base_id.to_string(),
            target_id: commit.id.clone(),
        };
    }

    HistorySelectionAction::ShowParentDiff {
        parent_id: commit.parent_id.clone(),
        target_id: commit.id.clone(),
    }
}

pub fn short_commit_id(commit_id: &str) -> String {
    commit_id.chars().take(7).collect()
}

#[cfg(test)]
mod tests {
    use super::{HistorySelectionAction, resolve_history_selection};
    use deve_core::source_control::CommitInfo;

    fn commit(id: &str, parent_id: Option<&str>) -> CommitInfo {
        CommitInfo {
            id: id.to_string(),
            parent_id: parent_id.map(str::to_string),
            message: format!("commit-{id}"),
            timestamp: 0,
            doc_count: 1,
            ledger_seq: 1,
        }
    }

    #[test]
    fn selected_commit_toggles_closed() {
        let commit = commit("bbbbbbb", Some("aaaaaaa"));
        assert_eq!(
            resolve_history_selection(Some("bbbbbbb"), None, &commit),
            HistorySelectionAction::ToggleClosed
        );
    }

    #[test]
    fn default_click_compares_parent_to_commit() {
        let commit = commit("bbbbbbb", Some("aaaaaaa"));
        assert_eq!(
            resolve_history_selection(None, None, &commit),
            HistorySelectionAction::ShowParentDiff {
                parent_id: Some("aaaaaaa".to_string()),
                target_id: "bbbbbbb".to_string(),
            }
        );
    }

    #[test]
    fn compare_mode_uses_selected_base_commit() {
        let base = commit("base123", Some("older01"));
        let target = commit("next456", Some("base123"));
        assert_eq!(
            resolve_history_selection(None, Some(base.id.as_str()), &target),
            HistorySelectionAction::ShowRangeDiff {
                base_id: "base123".to_string(),
                target_id: "next456".to_string(),
            }
        );
    }

    #[test]
    fn clicking_base_commit_clears_compare_mode() {
        let base = commit("base123", Some("older01"));
        assert_eq!(
            resolve_history_selection(None, Some(base.id.as_str()), &base),
            HistorySelectionAction::ClearBaseSelection
        );
    }
}

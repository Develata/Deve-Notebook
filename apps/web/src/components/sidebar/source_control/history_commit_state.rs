//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryCommitVisualState {
    Idle,
    Selected,
    Base,
    CompareTarget,
}

pub fn resolve_history_commit_visual_state(
    selected_commit_id: Option<&str>,
    compare_base_commit_id: Option<&str>,
    commit_id: &str,
) -> HistoryCommitVisualState {
    if compare_base_commit_id == Some(commit_id) {
        return HistoryCommitVisualState::Base;
    }

    if selected_commit_id == Some(commit_id) {
        if compare_base_commit_id.is_some() {
            return HistoryCommitVisualState::CompareTarget;
        }
        return HistoryCommitVisualState::Selected;
    }

    HistoryCommitVisualState::Idle
}

pub fn history_commit_row_class(
    state: HistoryCommitVisualState,
    read_blocked: bool,
) -> &'static str {
    match (state, read_blocked) {
        (HistoryCommitVisualState::Idle, true) => "pr-2 cursor-default",
        (HistoryCommitVisualState::Idle, false) => "pr-2 cursor-pointer",
        (HistoryCommitVisualState::Selected, true) => {
            "pr-2 cursor-default rounded bg-hover/80 ring-1 ring-default"
        }
        (HistoryCommitVisualState::Selected, false) => {
            "pr-2 cursor-pointer rounded bg-hover/80 ring-1 ring-default"
        }
        (HistoryCommitVisualState::Base, true) => {
            "pr-2 cursor-default rounded bg-accent/10 ring-1 ring-accent/20"
        }
        (HistoryCommitVisualState::Base, false) => {
            "pr-2 cursor-pointer rounded bg-accent/10 ring-1 ring-accent/20"
        }
        (HistoryCommitVisualState::CompareTarget, true) => {
            "pr-2 cursor-default rounded bg-hover/80 ring-1 ring-accent/30"
        }
        (HistoryCommitVisualState::CompareTarget, false) => {
            "pr-2 cursor-pointer rounded bg-hover/80 ring-1 ring-accent/30"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HistoryCommitVisualState, resolve_history_commit_visual_state};

    #[test]
    fn compare_target_is_distinguished_from_plain_selection() {
        assert_eq!(
            resolve_history_commit_visual_state(Some("target"), Some("base"), "target"),
            HistoryCommitVisualState::CompareTarget
        );
    }

    #[test]
    fn base_commit_takes_priority_over_selection() {
        assert_eq!(
            resolve_history_commit_visual_state(Some("base"), Some("base"), "base"),
            HistoryCommitVisualState::Base
        );
    }
}

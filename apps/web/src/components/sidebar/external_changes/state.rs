//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!
//! External Changes view state projection.

use crate::hooks::use_core::ExternalChangesContext;
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::hooks::use_core::write_gate_banner::reason_from_block;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

pub(super) fn should_request_external_changes(
    has_repo: bool,
    branch_switching: bool,
    repo_switching: bool,
    read_blocked: bool,
) -> bool {
    has_repo && !branch_switching && !repo_switching && !read_blocked
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExternalChangesVisibleState {
    Blocked(RepoWriteBlock),
    Empty,
    Changes,
}

pub(super) fn external_changes_visible_state(
    read_block: Option<RepoWriteBlock>,
    staged_count: usize,
    unstaged_count: usize,
) -> ExternalChangesVisibleState {
    if let Some(block) = read_block {
        return ExternalChangesVisibleState::Blocked(block);
    }
    if staged_count == 0 && unstaged_count == 0 {
        return ExternalChangesVisibleState::Empty;
    }
    ExternalChangesVisibleState::Changes
}

pub(super) fn can_apply_to_ledger(core: &ExternalChangesContext) -> bool {
    let staged = core.staged_changes.get();
    let unstaged = core.unstaged_changes.get();
    can_apply_to_ledger_state(core.can_write.get(), &staged, &unstaged)
}

pub(super) fn apply_title(locale: Locale, core: &ExternalChangesContext) -> String {
    let staged = core.staged_changes.get();
    let unstaged = core.unstaged_changes.get();

    if can_apply_to_ledger_state(core.can_write.get(), &staged, &unstaged) {
        return t::external_changes::apply_to_ledger(locale).to_string();
    }
    if let Some(block) = core.write_block.get() {
        return external_changes_blocked_hint(locale, block);
    }
    if external_changes_have_overlap(&staged, &unstaged) {
        return t::external_changes::overlap_blocked(locale).to_string();
    }
    t::external_changes::apply_to_ledger_disabled(locale).to_string()
}

pub(super) fn external_changes_blocked_hint(locale: Locale, block: RepoWriteBlock) -> String {
    let reason = t::write_gate::reason_label(locale, reason_from_block(block));
    t::external_changes::blocked_hint_with_reason(locale, reason)
}

pub(super) fn can_apply_to_ledger_state(
    can_write: bool,
    staged: &[ChangeEntry],
    unstaged: &[ChangeEntry],
) -> bool {
    can_write && !staged.is_empty() && !external_changes_have_overlap(staged, unstaged)
}

pub(super) fn external_changes_have_overlap(
    staged: &[ChangeEntry],
    unstaged: &[ChangeEntry],
) -> bool {
    staged
        .iter()
        .chain(unstaged.iter())
        .any(|entry| entry.has_conflict)
}

use super::section::{external_section_key, external_section_panel_id};
use super::state::{
    ExternalChangesVisibleState, can_apply_to_ledger_state, external_changes_blocked_hint,
    external_changes_have_overlap, external_changes_visible_state,
};
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::write_gate::WriteGateReason;
use crate::i18n::{Locale, t};
use deve_core::source_control::{ChangeEntry, ChangeStatus};

fn entry(path: &str, has_conflict: bool) -> ChangeEntry {
    ChangeEntry {
        path: path.into(),
        renamed_from: None,
        doc_id: None,
        status: ChangeStatus::Modified,
        has_conflict,
        domain: Default::default(),
        base_seq: None,
        target_seq: None,
    }
}

#[test]
fn external_changes_sections_use_stable_local_ids() {
    assert_eq!(external_section_key(true), "staged");
    assert_eq!(external_section_key(false), "pending");
    assert_eq!(
        external_section_panel_id(true),
        "external-changes-staged-panel"
    );
    assert_eq!(
        external_section_panel_id(false),
        "external-changes-pending-panel"
    );
}

#[test]
fn external_changes_section_headers_are_accessible_toggles() {
    let source = include_str!("section.rs");

    assert!(source.contains(concat!("data-deve-", "external-section-toggle")));
    assert!(source.contains(concat!("class=\"flex ", "h-11 w-full")));
    assert!(source.contains(concat!("md:", "h-7")));
    assert!(source.contains(concat!(
        "aria-expanded=move || ",
        "expanded.get().to_string()"
    )));
    assert!(source.contains(concat!("aria-controls=", "panel_id")));
    assert!(source.contains(concat!("data-deve-", "external-section-body")));
    assert!(source.contains(concat!("hidden=move || ", "!expanded.get()")));
    assert!(!source.contains(concat!("data-deve-", "sc-section-toggle")));
}

#[test]
fn external_changes_view_keeps_source_control_history_graph_out() {
    let source = include_str!("../external_changes.rs");

    assert!(source.contains(concat!("data-deve-", "external-changes-view")));
    assert!(!source.contains(concat!("source_control::", "History")));
    assert!(!source.contains(concat!("source_control::", "Graph")));
    assert!(!source.contains(concat!("source_control_", "history")));
    assert!(!source.contains(concat!("source_control_", "graph")));
    assert!(!source.contains(concat!("data-deve-sc-panel-body=", "\"history\"")));
    assert!(!source.contains(concat!("data-deve-sc-panel-body=", "\"graph\"")));
}

#[test]
fn external_changes_apply_label_is_not_commit() {
    let source = include_str!("../external_changes.rs");

    assert!(source.contains(concat!("data-deve-", "external-apply")));
    assert!(source.contains("t::external_changes::apply_to_ledger"));
    assert!(source.contains("core_for_apply_click.on_apply_to_ledger.run(())"));
    assert!(!source.contains("t::source_control::commit"));
    assert!(!source.contains("on_commit"));
}

#[test]
fn apply_to_ledger_fails_closed_when_any_external_change_overlaps() {
    let staged = vec![entry("clean.md", false)];
    let unstaged = vec![entry("overlap.md", true)];

    assert!(external_changes_have_overlap(&staged, &unstaged));
    assert!(!can_apply_to_ledger_state(true, &staged, &unstaged));
}

#[test]
fn blocked_external_changes_state_hides_empty_or_stale_rows() {
    assert_eq!(
        external_changes_visible_state(Some(RepoWriteBlock::ReadOnly), 0, 0),
        ExternalChangesVisibleState::Blocked(RepoWriteBlock::ReadOnly)
    );
    assert_eq!(
        external_changes_visible_state(Some(RepoWriteBlock::HandshakingRepo), 1, 0),
        ExternalChangesVisibleState::Blocked(RepoWriteBlock::HandshakingRepo)
    );
    assert_eq!(
        external_changes_visible_state(None, 0, 0),
        ExternalChangesVisibleState::Empty
    );
    assert_eq!(
        external_changes_visible_state(None, 0, 1),
        ExternalChangesVisibleState::Changes
    );
}

#[test]
fn external_changes_blocked_hint_uses_write_gate_reason_copy() {
    assert!(
        external_changes_blocked_hint(Locale::Zh, RepoWriteBlock::ReadOnly).contains(
            t::write_gate::reason_label(Locale::Zh, WriteGateReason::ReadOnly)
        )
    );
    assert!(
        external_changes_blocked_hint(Locale::En, RepoWriteBlock::HandshakingRepo).contains(
            t::write_gate::reason_label(Locale::En, WriteGateReason::HandshakingRepo)
        )
    );
}

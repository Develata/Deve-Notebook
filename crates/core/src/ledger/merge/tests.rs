// crates\core\src\ledger\merge\tests.rs
// ---------------------------------------------------------------
// 模块：三路合并测试
// 作用：验证 3-Way Merge 基本行为
// 功能：无冲突、可自动合并、冲突检测
// ---------------------------------------------------------------

use super::{MergeEngine, MergeResult};

#[test]
fn test_merge_no_conflict() {
    let base = "A\nB\nC";
    let local = "A\nB1\nC";
    let remote = "A\nB\nC"; // No change

    match MergeEngine::merge_commits(base, local, remote) {
        MergeResult::Success(content) => assert_eq!(content, "A\nB1\nC"),
        _ => panic!("Should be auto-merged"),
    }
}

#[test]
fn test_merge_auto_resolvable() {
    let base = "A\nB\nC";
    let local = "A1\nB\nC";
    let remote = "A\nB\nC1";

    match MergeEngine::merge_commits(base, local, remote) {
        MergeResult::Success(content) => {
            // dissimilar-based 3-way merge should auto-resolve
            assert!(content.contains("A1"));
            assert!(content.contains("C1"));
        }
        _ => panic!("Should be auto-merged"),
    }
}

#[test]
fn test_merge_conflict() {
    let base = "A\nB\nC";
    let local = "A\nB1\nC";
    let remote = "A\nB2\nC";

    match MergeEngine::merge_commits(base, local, remote) {
        MergeResult::Conflict {
            base: _,
            local: _,
            remote: _,
            conflicts,
        } => {
            assert!(!conflicts.is_empty());
        }
        _ => panic!("Should conflict"),
    }
}

#[test]
fn test_merge_conflicts_on_same_position_insertions() {
    let base = "AC";
    let local = "ABC";
    let remote = "AXC";

    match MergeEngine::merge_commits(base, local, remote) {
        MergeResult::Conflict { conflicts, .. } => {
            assert_eq!(conflicts.len(), 1);
            assert_eq!(conflicts[0].local_lines, vec!["B"]);
            assert_eq!(conflicts[0].remote_lines, vec!["X"]);
        }
        _ => panic!("Same-position divergent inserts must conflict"),
    }
}

#[test]
fn test_merge_conflicts_on_multi_edit_overlapping_region() {
    let base = "A\nB\nC\nD\nE";
    let local = "A\nB1\nC\nD1\nE";
    let remote = "A\nZ\nE";

    match MergeEngine::merge_commits(base, local, remote) {
        MergeResult::Conflict { conflicts, .. } => {
            assert_eq!(conflicts.len(), 1);
            assert_eq!(conflicts[0].local_lines, vec!["B1", "C", "D1"]);
            assert_eq!(conflicts[0].remote_lines, vec!["Z"]);
        }
        _ => panic!("Overlapping region with multiple local edits must conflict"),
    }
}

#[test]
fn test_merge_auto_resolves_non_overlapping_delete_and_insert() {
    let base = "A\nB\nC\nD";
    let local = "A\nB\nC\nD\nL";
    let remote = "A\nC\nD";

    match MergeEngine::merge_commits(base, local, remote) {
        MergeResult::Success(content) => assert_eq!(content, "A\nC\nD\nL"),
        _ => panic!("Non-overlapping delete and insert should auto-merge"),
    }
}

#[test]
fn test_merge_unicode_conflict_uses_character_indices() {
    let base = "甲\n乙\n丙";
    let local = "甲\n本地\n丙";
    let remote = "甲\n远端\n丙";

    match MergeEngine::merge_commits(base, local, remote) {
        MergeResult::Conflict { conflicts, .. } => {
            assert_eq!(conflicts.len(), 1);
            assert_eq!(conflicts[0].local_lines, vec!["本地"]);
            assert_eq!(conflicts[0].remote_lines, vec!["远端"]);
        }
        _ => panic!("Divergent unicode replacements must conflict"),
    }
}

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

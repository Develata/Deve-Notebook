use super::{normalize_executable_file_op_action, update_recent_move_dirs};
use crate::components::search_box::types::{FileOpAction, FileOpKind};
use leptos::prelude::*;

#[test]
fn file_op_action_execution_rejects_internal_repo_paths() {
    let op = FileOpAction {
        kind: FileOpKind::Move,
        src: "notes/readme.md".to_string(),
        dst: Some("notes/.git/config.md".to_string()),
    };

    assert!(normalize_executable_file_op_action(&op).is_none());
}

#[test]
fn file_op_action_execution_rejects_same_move_target() {
    let op = FileOpAction {
        kind: FileOpKind::Move,
        src: "notes/readme.md".to_string(),
        dst: Some("notes/readme.md".to_string()),
    };

    assert!(normalize_executable_file_op_action(&op).is_none());
}

#[test]
fn file_op_action_execution_rejects_normalized_same_move_target() {
    let op = FileOpAction {
        kind: FileOpKind::Move,
        src: "notes/readme.md".to_string(),
        dst: Some("notes\\readme".to_string()),
    };

    assert!(normalize_executable_file_op_action(&op).is_none());
}

#[test]
fn file_op_action_execution_normalizes_dispatch_paths() {
    let op = FileOpAction {
        kind: FileOpKind::Copy,
        src: "notes\\readme".to_string(),
        dst: Some("archive\\readme".to_string()),
    };

    let normalized = normalize_executable_file_op_action(&op).expect("valid file op");

    assert_eq!(normalized.src, "notes/readme.md");
    assert_eq!(normalized.dst.as_deref(), Some("archive/readme.md"));
}

#[test]
fn file_op_action_execution_rejects_missing_move_destination() {
    let op = FileOpAction {
        kind: FileOpKind::Move,
        src: "notes/readme.md".to_string(),
        dst: None,
    };

    assert!(normalize_executable_file_op_action(&op).is_none());
}

#[test]
fn file_op_action_execution_accepts_valid_file_ops() {
    let mv = FileOpAction {
        kind: FileOpKind::Move,
        src: "notes/readme.md".to_string(),
        dst: Some("archive/readme.md".to_string()),
    };
    let rm = FileOpAction {
        kind: FileOpKind::Remove,
        src: "notes/readme.md".to_string(),
        dst: None,
    };

    assert!(normalize_executable_file_op_action(&mv).is_some());
    assert!(normalize_executable_file_op_action(&rm).is_some());
}

#[test]
fn file_op_action_execution_rejects_directory_remove() {
    let op = FileOpAction {
        kind: FileOpKind::Remove,
        src: "notes/".to_string(),
        dst: None,
    };

    assert!(normalize_executable_file_op_action(&op).is_none());
}

#[test]
fn file_op_action_execution_rejects_directory_move_source() {
    let op = FileOpAction {
        kind: FileOpKind::Move,
        src: "notes/".to_string(),
        dst: Some("archive/notes.md".to_string()),
    };

    assert!(normalize_executable_file_op_action(&op).is_none());
}

#[test]
fn file_op_action_execution_rejects_directory_copy_destination() {
    let op = FileOpAction {
        kind: FileOpKind::Copy,
        src: "notes/readme.md".to_string(),
        dst: Some("archive/".to_string()),
    };

    assert!(normalize_executable_file_op_action(&op).is_none());
}

#[test]
fn recent_move_dirs_uses_shared_forward_slash_policy() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let (recent_move_dirs, set_recent_move_dirs) = signal(Vec::new());

        update_recent_move_dirs(set_recent_move_dirs, "archive\\nested\\readme.md");

        assert_eq!(
            recent_move_dirs.get_untracked(),
            vec!["archive/nested/".to_string()]
        );
    });
}

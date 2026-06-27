//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::components::search_box::file_ops;
use crate::components::search_box::providers::LOCAL_BRANCH_LABEL;
use crate::components::search_box::runtime::SearchRuntime;
use crate::components::search_box::types::{FileOpAction, FileOpKind, InsertQuery, SearchAction};
use leptos::prelude::*;

use super::write_gate_feedback::allow_repo_write;

pub(crate) fn execute_action(
    action: &SearchAction,
    runtime: &SearchRuntime,
    set_show: WriteSignal<bool>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
    input_ref: NodeRef<leptos::html::Input>,
    set_recent_move_dirs: WriteSignal<Vec<String>>,
) {
    match action {
        SearchAction::OpenDoc(id) => {
            runtime.document.on_doc_select.run(*id);
            set_show.set(false);
        }
        SearchAction::RunCommand(cmd) => cmd.action.run(()),
        SearchAction::SwitchBranch(branch) => {
            if branch == LOCAL_BRANCH_LABEL {
                runtime.branch.on_switch_branch.run(None);
            } else {
                runtime.branch.on_switch_branch.run(Some(branch.clone()));
            }
            set_show.set(false);
        }
        SearchAction::CreateDoc(path) => {
            if !allow_repo_write(runtime, "create document") {
                return;
            }
            let path = path.trim();
            if path.is_empty() {
                return;
            }
            if file_ops::validate_doc_create_path(path).is_some() {
                return;
            }
            runtime
                .document
                .on_doc_create
                .run(file_ops::normalize_doc_path(path));
            set_show.set(false);
        }
        SearchAction::FileOp(op) => {
            if !file_op_action_is_executable(op) {
                return;
            }
            match op.kind {
                FileOpKind::Move => {
                    if !allow_repo_write(runtime, "move document") {
                        return;
                    }
                    if let Some(dst) = &op.dst {
                        runtime
                            .document
                            .on_doc_move
                            .run((op.src.clone(), dst.clone()));
                        update_recent_move_dirs(set_recent_move_dirs, dst);
                        set_show.set(false);
                    }
                }
                FileOpKind::Copy => {
                    if !allow_repo_write(runtime, "copy document") {
                        return;
                    }
                    if let Some(dst) = &op.dst {
                        runtime
                            .document
                            .on_doc_copy
                            .run((op.src.clone(), dst.clone()));
                        set_show.set(false);
                    }
                }
                FileOpKind::Remove => {
                    if !allow_repo_write(runtime, "delete document") {
                        return;
                    }
                    runtime.document.on_doc_delete.run(op.src.clone());
                    set_show.set(false);
                }
            }
        }
        SearchAction::InsertQuery(InsertQuery { query, cursor }) => {
            set_query.set(query.clone());
            set_selected_index.set(0);
            let cursor = *cursor;
            request_animation_frame(move || {
                if let Some(el) = input_ref.get_untracked() {
                    let _ = el.set_selection_range(cursor as u32, cursor as u32);
                }
            });
        }
        SearchAction::Noop => {}
    }
}

fn update_recent_move_dirs(set_recent_move_dirs: WriteSignal<Vec<String>>, dst: &str) {
    let normalized = dst.replace('\\', "/");
    let parent = std::path::Path::new(&normalized)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    if parent.is_empty() {
        return;
    }
    let dir = format!("{}/", parent.replace('\\', "/"));
    set_recent_move_dirs.update(|list| {
        list.retain(|d| d != &dir);
        list.insert(0, dir);
        if list.len() > 4 {
            list.truncate(4);
        }
    });
}

fn file_op_action_is_executable(op: &FileOpAction) -> bool {
    if file_ops::validate_doc_shell_path(&op.src).is_some() {
        return false;
    }

    match op.kind {
        FileOpKind::Move | FileOpKind::Copy => {
            let Some(dst) = op.dst.as_deref() else {
                return false;
            };
            file_ops::validate_doc_shell_path(dst).is_none() && dst != op.src
        }
        FileOpKind::Remove => {
            op.dst.is_none() && file_ops::validate_doc_file_path(&op.src).is_none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::file_op_action_is_executable;
    use crate::components::search_box::types::{FileOpAction, FileOpKind};

    #[test]
    fn file_op_action_execution_rejects_internal_repo_paths() {
        let op = FileOpAction {
            kind: FileOpKind::Move,
            src: "notes/readme.md".to_string(),
            dst: Some("notes/.git/config.md".to_string()),
        };

        assert!(!file_op_action_is_executable(&op));
    }

    #[test]
    fn file_op_action_execution_rejects_same_move_target() {
        let op = FileOpAction {
            kind: FileOpKind::Move,
            src: "notes/readme.md".to_string(),
            dst: Some("notes/readme.md".to_string()),
        };

        assert!(!file_op_action_is_executable(&op));
    }

    #[test]
    fn file_op_action_execution_rejects_missing_move_destination() {
        let op = FileOpAction {
            kind: FileOpKind::Move,
            src: "notes/readme.md".to_string(),
            dst: None,
        };

        assert!(!file_op_action_is_executable(&op));
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

        assert!(file_op_action_is_executable(&mv));
        assert!(file_op_action_is_executable(&rm));
    }

    #[test]
    fn file_op_action_execution_rejects_directory_remove() {
        let op = FileOpAction {
            kind: FileOpKind::Remove,
            src: "notes/".to_string(),
            dst: None,
        };

        assert!(!file_op_action_is_executable(&op));
    }
}

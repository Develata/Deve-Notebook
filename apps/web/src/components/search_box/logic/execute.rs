use crate::components::search_box::file_ops;
use crate::components::search_box::types::{InsertQuery, SearchAction};
use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

pub(crate) fn execute_action(
    action: &SearchAction,
    core: &CoreState,
    set_show: WriteSignal<bool>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
    input_ref: NodeRef<leptos::html::Input>,
    set_recent_move_dirs: WriteSignal<Vec<String>>,
) {
    match action {
        SearchAction::OpenDoc(id) => {
            core.on_doc_select.run(*id);
            set_show.set(false);
        }
        SearchAction::RunCommand(cmd) => cmd.action.run(()),
        SearchAction::SwitchBranch(branch) => {
            if branch == "Local (Master)" {
                core.on_switch_branch.run(None);
            } else {
                core.on_switch_branch.run(Some(branch.clone()));
            }
            set_show.set(false);
        }
        SearchAction::CreateDoc(path) => {
            core.on_doc_create.run(file_ops::normalize_doc_path(path));
            set_show.set(false);
        }
        SearchAction::FileOp(op) => match op.kind {
            crate::components::search_box::types::FileOpKind::Move => {
                if let Some(dst) = &op.dst {
                    core.on_doc_move.run((op.src.clone(), dst.clone()));
                    update_recent_move_dirs(set_recent_move_dirs, dst);
                    set_show.set(false);
                }
            }
            crate::components::search_box::types::FileOpKind::Copy => {
                if let Some(dst) = &op.dst {
                    core.on_doc_copy.run((op.src.clone(), dst.clone()));
                    set_show.set(false);
                }
            }
            crate::components::search_box::types::FileOpKind::Remove => {
                core.on_doc_delete.run(op.src.clone());
                set_show.set(false);
            }
        },
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

use super::{build_new_window_url, build_rename_prefill, create_action_handler};
use crate::context_action::{
    ContextActionId, ContextActionIntent, ContextActionSurface, ContextActionTarget,
};
use leptos::prelude::{Callable, Callback, GetUntracked, Set, Update, signal};

fn file_tree_intent(action_id: ContextActionId, path: &str) -> ContextActionIntent {
    ContextActionIntent::new(
        action_id,
        ContextActionSurface::FileTree,
        ContextActionTarget::from_file_tree_node(false, path),
    )
}

fn command_palette_intent(action_id: ContextActionId, path: &str) -> ContextActionIntent {
    ContextActionIntent::new(
        action_id,
        ContextActionSurface::CommandPalette,
        ContextActionTarget::from_file_tree_node(false, path),
    )
}

#[test]
fn rename_prefill_keeps_file_in_same_dir() {
    assert_eq!(
        build_rename_prefill("123/324.md"),
        ">mv \"123/324.md\" \"123/324|.md\""
    );
}

#[test]
fn rename_prefill_keeps_folder_without_extension() {
    assert_eq!(
        build_rename_prefill("ABC/modals"),
        ">mv \"ABC/modals\" \"ABC/modals|\""
    );
}

#[test]
fn new_window_url_appends_doc_to_existing_query() {
    assert_eq!(
        build_new_window_url("http://127.0.0.1:8080/?sc_full=1", "123%2F324.md"),
        "http://127.0.0.1:8080/?sc_full=1&doc=123%2F324.md"
    );
}

#[test]
fn new_window_url_preserves_hash_fragment() {
    assert_eq!(
        build_new_window_url("http://127.0.0.1:8080/#section", "doc.md"),
        "http://127.0.0.1:8080/?doc=doc.md#section"
    );
}

#[test]
fn new_window_url_replaces_stale_doc_query_param() {
    assert_eq!(
        build_new_window_url(
            "http://127.0.0.1:8080/?doc=old.md&sc_full=1#section",
            "notes%2Fnew.md"
        ),
        "http://127.0.0.1:8080/?sc_full=1&doc=notes%2Fnew.md#section"
    );
}

#[test]
fn export_pdf_action_handler_is_fail_closed_without_side_effects() {
    let (is_readonly, _) = signal(false);
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(is_readonly.into(), delete_req, open_search);

    handler.run(file_tree_intent(
        ContextActionId::ExportPdf,
        "notes/readme.md",
    ));

    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

#[test]
fn surface_mismatch_action_handler_is_fail_closed_without_side_effects() {
    let (is_readonly, _) = signal(false);
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(is_readonly.into(), delete_req, open_search);

    handler.run(command_palette_intent(
        ContextActionId::Delete,
        "notes/readme.md",
    ));

    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

#[test]
fn readonly_current_state_blocks_write_action_handler_side_effects() {
    let (is_readonly, set_readonly) = signal(false);
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(is_readonly.into(), delete_req, open_search);
    set_readonly.set(true);

    handler.run(file_tree_intent(ContextActionId::Rename, "notes/readme.md"));

    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

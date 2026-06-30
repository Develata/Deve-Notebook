use super::{
    build_new_window_url, build_prefill_command, build_rename_prefill, create_action_handler,
};
use crate::context_action::{
    ContextActionId, ContextActionIntent, ContextActionReadiness, ContextActionScope,
    ContextActionSurface, ContextActionTarget,
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

fn scoped_file_tree_intent(
    action_id: ContextActionId,
    path: &str,
    scope: ContextActionScope,
) -> ContextActionIntent {
    ContextActionIntent::with_scope(
        action_id,
        ContextActionSurface::FileTree,
        ContextActionTarget::from_file_tree_node(false, path),
        scope,
    )
}

fn no_op_host_action() -> Callback<String> {
    Callback::new(|_: String| {})
}

#[test]
fn rename_prefill_keeps_file_in_same_dir() {
    assert_eq!(
        build_rename_prefill("123/324.md").as_deref(),
        Some(">mv \"123/324.md\" \"123/324|.md\"")
    );
}

#[test]
fn rename_prefill_keeps_folder_without_extension() {
    assert_eq!(
        build_rename_prefill("ABC/modals").as_deref(),
        Some(">mv \"ABC/modals\" \"ABC/modals|\"")
    );
}

#[test]
fn rename_prefill_rejects_unrepresentable_shell_path() {
    assert_eq!(build_rename_prefill("notes/a|b.md"), None);
    assert_eq!(build_rename_prefill("notes/a\"b.md"), None);
}

#[test]
fn prefill_command_rejects_ambiguous_cursor_marker() {
    assert_eq!(
        build_prefill_command("mv", "notes/a.md", Some("notes/a||.md".to_string())),
        None
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
fn new_window_url_replaces_percent_encoded_doc_query_key() {
    assert_eq!(
        build_new_window_url(
            "http://127.0.0.1:8080/?do%63=old.md&sc_full=1#section",
            "notes%2Fnew.md"
        ),
        "http://127.0.0.1:8080/?sc_full=1&doc=notes%2Fnew.md#section"
    );
}

#[test]
fn export_pdf_action_handler_is_fail_closed_without_side_effects() {
    let (readiness, _) = signal(ContextActionReadiness::from_readonly(false));
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(
        readiness.into(),
        delete_req,
        open_search,
        no_op_host_action(),
        no_op_host_action(),
    );

    handler.run(file_tree_intent(
        ContextActionId::ExportPdf,
        "notes/readme.md",
    ));

    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

#[test]
fn surface_mismatch_action_handler_is_fail_closed_without_side_effects() {
    let (readiness, _) = signal(ContextActionReadiness::from_readonly(false));
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(
        readiness.into(),
        delete_req,
        open_search,
        no_op_host_action(),
        no_op_host_action(),
    );

    handler.run(command_palette_intent(
        ContextActionId::Delete,
        "notes/readme.md",
    ));

    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

#[test]
fn readonly_current_state_blocks_write_action_handler_side_effects() {
    let (readiness, set_readiness) = signal(ContextActionReadiness::from_readonly(false));
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(
        readiness.into(),
        delete_req,
        open_search,
        no_op_host_action(),
        no_op_host_action(),
    );
    set_readiness.set(ContextActionReadiness::from_readonly(true));

    handler.run(file_tree_intent(ContextActionId::Rename, "notes/readme.md"));

    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

#[test]
fn write_blocked_current_state_blocks_write_action_handler_side_effects() {
    let (readiness, set_readiness) = signal(ContextActionReadiness::from_readonly(false));
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(
        readiness.into(),
        delete_req,
        open_search,
        no_op_host_action(),
        no_op_host_action(),
    );
    set_readiness.set(ContextActionReadiness::from_readonly(false).with_write_blocked(true));

    handler.run(file_tree_intent(ContextActionId::Delete, "notes/readme.md"));

    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

#[test]
fn scope_change_blocks_stale_action_handler_intent_side_effects() {
    let projected_scope = ContextActionScope::new(Some("repo-a".to_string()), 1);
    let current_scope = ContextActionScope::new(Some("repo-a".to_string()), 2);
    let (readiness, set_readiness) =
        signal(ContextActionReadiness::from_readonly(false).with_scope(projected_scope.clone()));
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(
        readiness.into(),
        delete_req,
        open_search,
        no_op_host_action(),
        no_op_host_action(),
    );
    set_readiness.set(ContextActionReadiness::from_readonly(false).with_scope(current_scope));

    handler.run(scoped_file_tree_intent(
        ContextActionId::Rename,
        "notes/readme.md",
        projected_scope,
    ));

    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

#[test]
fn unrepresentable_path_blocks_search_prefill_side_effects() {
    let (readiness, _) = signal(ContextActionReadiness::from_readonly(false));
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(
        readiness.into(),
        delete_req,
        open_search,
        no_op_host_action(),
        no_op_host_action(),
    );

    handler.run(file_tree_intent(ContextActionId::Rename, "notes/a|b.md"));

    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

#[test]
fn internal_repo_path_blocks_action_handler_side_effects() {
    let (readiness, _) = signal(ContextActionReadiness::from_readonly(false));
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(
        readiness.into(),
        delete_req,
        open_search,
        no_op_host_action(),
        no_op_host_action(),
    );

    handler.run(file_tree_intent(
        ContextActionId::Rename,
        "notes/.git/config.md",
    ));

    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

#[test]
fn file_tree_action_host_file_handler_dispatches_supported_readonly_callbacks() {
    let (readiness, _) =
        signal(ContextActionReadiness::from_readonly(true).with_host_file_actions(true, true));
    let (delete_count, set_delete_count) = signal(0);
    let (search_count, set_search_count) = signal(0);
    let (copy_paths, set_copy_paths) = signal(Vec::<String>::new());
    let (reveal_paths, set_reveal_paths) = signal(Vec::<String>::new());
    let delete_req = Callback::new(move |_: String| {
        set_delete_count.update(|count| *count += 1);
    });
    let open_search = Callback::new(move |_: String| {
        set_search_count.update(|count| *count += 1);
    });
    let copy_absolute_path = Callback::new(move |path: String| {
        set_copy_paths.update(|paths| paths.push(path));
    });
    let reveal_in_system_explorer = Callback::new(move |path: String| {
        set_reveal_paths.update(|paths| paths.push(path));
    });

    let handler = create_action_handler(
        readiness.into(),
        delete_req,
        open_search,
        copy_absolute_path,
        reveal_in_system_explorer,
    );

    handler.run(file_tree_intent(
        ContextActionId::CopyAbsolutePath,
        "notes/readme.md",
    ));
    handler.run(file_tree_intent(
        ContextActionId::RevealInSystemExplorer,
        "notes/readme.md",
    ));

    assert_eq!(copy_paths.get_untracked(), vec!["notes/readme.md"]);
    assert_eq!(reveal_paths.get_untracked(), vec!["notes/readme.md"]);
    assert_eq!(delete_count.get_untracked(), 0);
    assert_eq!(search_count.get_untracked(), 0);
}

#[test]
fn file_tree_action_host_file_handler_rechecks_stale_capability() {
    let (readiness, set_readiness) =
        signal(ContextActionReadiness::from_readonly(false).with_host_file_actions(true, true));
    let (copy_count, set_copy_count) = signal(0);
    let delete_req = Callback::new(|_: String| {});
    let open_search = Callback::new(|_: String| {});
    let copy_absolute_path = Callback::new(move |_: String| {
        set_copy_count.update(|count| *count += 1);
    });

    let handler = create_action_handler(
        readiness.into(),
        delete_req,
        open_search,
        copy_absolute_path,
        no_op_host_action(),
    );
    set_readiness.set(ContextActionReadiness::from_readonly(false));

    handler.run(file_tree_intent(
        ContextActionId::CopyAbsolutePath,
        "notes/readme.md",
    ));

    assert_eq!(copy_count.get_untracked(), 0);
}

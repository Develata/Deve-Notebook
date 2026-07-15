use crate::components::editor_tabs::{
    EditorTabKey,
    close::close_diff_tab,
    diff_tab_from_session,
    ops::{remove_diff_tab, remove_diff_tab_with_order, upsert_diff_tab},
};
use crate::runtime::source_control_client::diff_session::DiffSessionWire;
use deve_core::models::DocId;
use leptos::prelude::{GetUntracked, signal};

#[test]
fn removing_diff_tab_returns_neighbor_session() {
    let mut tabs = vec![
        diff_tab_from_session(DiffSessionWire::new(
            "a.md".into(),
            "old".into(),
            "new".into(),
        )),
        diff_tab_from_session(DiffSessionWire::new(
            "b.md".into(),
            "old".into(),
            "new".into(),
        )),
    ];
    let first_key = tabs[0].key.clone();
    let second_path = tabs[1].session.path.clone();

    assert_eq!(
        remove_diff_tab(&mut tabs, &first_key)
            .expect("neighbor")
            .path,
        second_path
    );
}

#[test]
fn removing_diff_tab_with_visible_order_returns_right_neighbor_then_left_neighbor() {
    let first = diff_tab_from_session(DiffSessionWire::new(
        "a.md".into(),
        "old".into(),
        "new".into(),
    ));
    let second = diff_tab_from_session(DiffSessionWire::new(
        "b.md".into(),
        "old".into(),
        "new".into(),
    ));
    let third = diff_tab_from_session(DiffSessionWire::new(
        "c.md".into(),
        "old".into(),
        "new".into(),
    ));
    let first_key = first.key.clone();
    let second_key = second.key.clone();
    let third_key = third.key.clone();
    let mut tabs = vec![first, second, third];
    let mut visible_order = vec![
        EditorTabKey::Diff(third_key.clone()),
        EditorTabKey::Diff(first_key.clone()),
        EditorTabKey::Diff(second_key.clone()),
    ];

    assert_eq!(
        remove_diff_tab_with_order(&mut tabs, &mut visible_order, &first_key)
            .expect("right neighbor")
            .path,
        "b.md"
    );
    assert_eq!(
        remove_diff_tab_with_order(&mut tabs, &mut visible_order, &second_key)
            .expect("left neighbor")
            .path,
        "c.md"
    );
    assert!(remove_diff_tab_with_order(&mut tabs, &mut visible_order, &third_key).is_none());
}

#[test]
fn mobile_surface_close_diff_keeps_source_control_state() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let first = diff_tab_from_session(DiffSessionWire::new(
        "a.md".into(),
        "old-a".into(),
        "new-a".into(),
    ));
    let second = diff_tab_from_session(DiffSessionWire::new(
        "b.md".into(),
        "old-b".into(),
        "new-b".into(),
    ));
    let active_key = first.key.clone();
    let source_control_state = (1usize, 2usize, "commit message".to_string());
    let (source_control_state_signal, _set_source_control_state) =
        signal(source_control_state.clone());
    let (diff_content, set_diff_content) = signal(Some(first.session.clone()));
    let first_key = first.key.clone();
    let (diff_tabs, set_diff_tabs) = signal(vec![first, second.clone()]);
    let (tab_order, set_tab_order) = signal(vec![
        EditorTabKey::Diff(first_key),
        EditorTabKey::Diff(second.key.clone()),
    ]);

    close_diff_tab(
        active_key,
        diff_content,
        set_diff_content,
        diff_tabs,
        set_diff_tabs,
        tab_order,
        set_tab_order,
    );

    assert_eq!(
        diff_content
            .get_untracked()
            .as_ref()
            .map(|session| session.path.as_str()),
        Some(second.session.path.as_str())
    );
    assert_eq!(
        source_control_state_signal.get_untracked(),
        source_control_state
    );
}

#[test]
fn loading_and_resolved_doc_sessions_upsert_and_close_as_one_tab() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let doc_id = DocId::from_u128(21);
    let mut loading = DiffSessionWire::loading("notes/a.md".into(), "notes/a.md".into())
        .with_doc_id(Some(doc_id));
    loading.opened_at_ms = 41;
    let mut resolved = DiffSessionWire::new("notes/a.md".into(), "old".into(), "new".into())
        .with_doc_id(Some(doc_id));
    resolved.opened_at_ms = 42;
    let loading_tab = diff_tab_from_session(loading);
    let resolved_tab = diff_tab_from_session(resolved.clone());

    assert_eq!(loading_tab.key, resolved_tab.key);
    assert_eq!(resolved_tab.key, format!("doc:{doc_id}"));

    let mut model_tabs = vec![loading_tab];
    upsert_diff_tab(&mut model_tabs, resolved_tab.clone());
    assert_eq!(model_tabs.len(), 1);
    assert_eq!(model_tabs[0].session, resolved);

    let active_key = resolved_tab.key.clone();
    let (diff_content, set_diff_content) = signal(Some(resolved_tab.session));
    let (diff_tabs, set_diff_tabs) = signal(model_tabs);
    let (tab_order, set_tab_order) = signal(vec![EditorTabKey::Diff(active_key.clone())]);

    close_diff_tab(
        active_key,
        diff_content,
        set_diff_content,
        diff_tabs,
        set_diff_tabs,
        tab_order,
        set_tab_order,
    );

    assert!(diff_content.get_untracked().is_none());
    assert!(diff_tabs.get_untracked().is_empty());
    assert!(tab_order.get_untracked().is_empty());
}

#[test]
fn history_cache_hit_and_miss_share_doc_tab_identity() {
    let doc_id = DocId::from_u128(22);
    let cache_key = "base..target:notes/a.md".to_string();
    let cache_miss = DiffSessionWire::loading("notes/a.md".into(), "old.md -> a.md".into())
        .with_doc_id(Some(doc_id))
        .with_pending_request("request-1".into())
        .with_cache_key(cache_key.clone());
    let cache_hit = DiffSessionWire::with_display_path(
        "notes/a.md".into(),
        "old.md -> a.md".into(),
        "old".into(),
        "new".into(),
    )
    .with_doc_id(Some(doc_id))
    .with_cache_key(cache_key);

    let miss_tab = diff_tab_from_session(cache_miss);
    let hit_tab = diff_tab_from_session(cache_hit);

    assert_eq!(miss_tab.key, hit_tab.key);
    assert_eq!(hit_tab.key, format!("doc:{doc_id}"));
}

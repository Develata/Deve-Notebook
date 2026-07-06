use crate::components::editor_tabs::{
    EditorTabKey,
    close::close_diff_tab,
    diff_tab_from_session,
    ops::{remove_diff_tab, remove_diff_tab_with_order},
};
use crate::runtime::source_control_client::diff_session::DiffSessionWire;
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

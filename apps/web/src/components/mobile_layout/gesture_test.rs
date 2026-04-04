use super::gesture::{SwipeTarget, resolve_swipe_target};

#[test]
fn edge_swipe_ignores_interactive_targets() {
    let target = resolve_swipe_target(490, 500, false, false, true);
    assert_eq!(target, None);
}

#[test]
fn edge_swipe_still_opens_outline_on_bare_edge_drag() {
    let target = resolve_swipe_target(490, 500, false, false, false);
    assert_eq!(target, Some(SwipeTarget::OpenRight));
}

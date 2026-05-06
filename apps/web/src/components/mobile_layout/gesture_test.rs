use super::gesture::{SwipeOutcome, SwipeTarget, resolve_swipe_outcome, resolve_swipe_target};

#[test]
fn mobile_drawer_edge_swipe_opens_left_from_left_edge() {
    let target = resolve_swipe_target(10, 500, false, false, false);
    assert_eq!(target, Some(SwipeTarget::OpenLeft));
    assert_eq!(resolve_swipe_outcome(target, 80), SwipeOutcome::OpenLeft);
}

#[test]
fn mobile_drawer_edge_swipe_opens_right_from_right_edge() {
    let target = resolve_swipe_target(490, 500, false, false, false);
    assert_eq!(target, Some(SwipeTarget::OpenRight));
    assert_eq!(resolve_swipe_outcome(target, -80), SwipeOutcome::OpenRight);
}

#[test]
fn mobile_drawer_edge_swipe_closes_open_drawers() {
    assert_eq!(
        resolve_swipe_outcome(Some(SwipeTarget::CloseLeft), -80),
        SwipeOutcome::CloseDrawers
    );
    assert_eq!(
        resolve_swipe_outcome(Some(SwipeTarget::CloseRight), 80),
        SwipeOutcome::CloseDrawers
    );
}

#[test]
fn mobile_drawer_edge_swipe_ignores_short_drags() {
    assert_eq!(
        resolve_swipe_outcome(Some(SwipeTarget::OpenLeft), 49),
        SwipeOutcome::None
    );
    assert_eq!(
        resolve_swipe_outcome(Some(SwipeTarget::OpenRight), -49),
        SwipeOutcome::None
    );
}

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

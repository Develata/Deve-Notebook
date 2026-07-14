use super::edge_swipe_left_drawer_view;
use super::gesture::{
    EDGE_SWIPE_BLOCKING_SELECTOR, SwipeOutcome, SwipeSession, SwipeTarget, TouchPoint,
    clear_swipe_session, resolve_swipe_outcome, resolve_swipe_start, resolve_touch_end_outcome,
};
use crate::components::activity_bar::SidebarView;

fn point(x: i32, y: i32) -> TouchPoint {
    TouchPoint { x, y }
}

#[test]
fn mobile_drawer_edge_swipe_opens_left_from_left_edge() {
    let session = resolve_swipe_start(point(10, 200), 500, false, false, false, 1);
    assert_eq!(
        session.map(|value| value.target),
        Some(SwipeTarget::OpenLeft)
    );
    assert_eq!(
        resolve_swipe_outcome(session, point(90, 205)),
        SwipeOutcome::OpenLeft
    );
}

#[test]
fn mobile_drawer_edge_swipe_opens_file_tree_instead_of_previous_left_view() {
    assert_eq!(edge_swipe_left_drawer_view(), SidebarView::Explorer);
}

#[test]
fn mobile_drawer_edge_swipe_opens_right_from_right_edge() {
    let session = resolve_swipe_start(point(490, 200), 500, false, false, false, 1);
    assert_eq!(
        session.map(|value| value.target),
        Some(SwipeTarget::OpenRight)
    );
    assert_eq!(
        resolve_swipe_outcome(session, point(410, 195)),
        SwipeOutcome::OpenRight
    );
}

#[test]
fn mobile_drawer_edge_swipe_closes_open_drawers() {
    assert_eq!(
        resolve_swipe_outcome(
            Some(SwipeSession::new(SwipeTarget::CloseLeft, point(200, 200))),
            point(120, 205),
        ),
        SwipeOutcome::CloseLeft
    );
    assert_eq!(
        resolve_swipe_outcome(
            Some(SwipeSession::new(SwipeTarget::CloseRight, point(200, 200))),
            point(280, 195),
        ),
        SwipeOutcome::CloseRight
    );
}

#[test]
fn mobile_drawer_edge_swipe_ignores_short_drags() {
    let left = Some(SwipeSession::new(SwipeTarget::OpenLeft, point(10, 200)));
    let right = Some(SwipeSession::new(SwipeTarget::OpenRight, point(490, 200)));
    assert_eq!(
        resolve_swipe_outcome(left, point(59, 200)),
        SwipeOutcome::None
    );
    assert_eq!(
        resolve_swipe_outcome(right, point(441, 200)),
        SwipeOutcome::None
    );
}

#[test]
fn mobile_drawer_edge_swipe_uses_exact_edge_and_distance_boundaries() {
    assert!(resolve_swipe_start(point(20, 200), 500, false, false, false, 1).is_some());
    assert!(resolve_swipe_start(point(21, 200), 500, false, false, false, 1).is_none());
    assert!(resolve_swipe_start(point(480, 200), 500, false, false, false, 1).is_some());
    assert!(resolve_swipe_start(point(479, 200), 500, false, false, false, 1).is_none());

    let session = Some(SwipeSession::new(SwipeTarget::OpenLeft, point(10, 200)));
    assert_eq!(
        resolve_swipe_outcome(session, point(60, 200)),
        SwipeOutcome::OpenLeft
    );
}

#[test]
fn mobile_drawer_edge_swipe_ignores_vertical_and_diagonal_drags() {
    let session = Some(SwipeSession::new(SwipeTarget::OpenLeft, point(10, 100)));
    assert_eq!(
        resolve_swipe_outcome(session, point(70, 190)),
        SwipeOutcome::None
    );
    assert_eq!(
        resolve_swipe_outcome(session, point(90, 180)),
        SwipeOutcome::None
    );
}

#[test]
fn mobile_drawer_edge_swipe_ignores_interactive_targets() {
    let session = resolve_swipe_start(point(490, 200), 500, false, false, true, 1);
    assert_eq!(session, None);
}

#[test]
fn mobile_drawer_edge_swipe_still_opens_outline_on_bare_edge_drag() {
    let session = resolve_swipe_start(point(490, 200), 500, false, false, false, 1);
    assert_eq!(
        session.map(|value| value.target),
        Some(SwipeTarget::OpenRight)
    );
}

#[test]
fn mobile_drawer_edge_swipe_allows_editor_content_start() {
    assert!(!EDGE_SWIPE_BLOCKING_SELECTOR.contains("contenteditable"));
    assert!(EDGE_SWIPE_BLOCKING_SELECTOR.contains("button"));
    assert!(EDGE_SWIPE_BLOCKING_SELECTOR.contains("data-no-edge-swipe"));

    let session = resolve_swipe_start(point(10, 200), 500, false, false, false, 1);
    assert_eq!(
        session.map(|value| value.target),
        Some(SwipeTarget::OpenLeft)
    );
    let session = resolve_swipe_start(point(490, 200), 500, false, false, false, 1);
    assert_eq!(
        session.map(|value| value.target),
        Some(SwipeTarget::OpenRight)
    );
}

#[test]
fn mobile_drawer_edge_swipe_rejects_multitouch_start() {
    let session = resolve_swipe_start(point(10, 200), 500, false, false, false, 2);
    assert_eq!(session, None);
}

#[test]
fn mobile_drawer_edge_swipe_touch_end_without_changed_touch_clears_capture() {
    let session = Some(SwipeSession::new(SwipeTarget::OpenLeft, point(10, 200)));
    let (outcome, next_session) = resolve_touch_end_outcome(session, None, 0);

    assert_eq!(outcome, SwipeOutcome::None);
    assert_eq!(next_session, None);
}

#[test]
fn mobile_drawer_edge_swipe_touch_end_with_remaining_touches_clears_capture() {
    let session = Some(SwipeSession::new(SwipeTarget::OpenLeft, point(10, 200)));
    let (outcome, next_session) = resolve_touch_end_outcome(session, Some(point(90, 200)), 1);

    assert_eq!(outcome, SwipeOutcome::None);
    assert_eq!(next_session, None);
}

#[test]
fn mobile_drawer_edge_swipe_touch_cancel_clears_capture() {
    let mut session = Some(SwipeSession::new(SwipeTarget::OpenLeft, point(10, 200)));
    clear_swipe_session(&mut session);
    assert_eq!(session, None);
}

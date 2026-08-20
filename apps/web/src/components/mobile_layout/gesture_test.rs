use super::edge_swipe_left_drawer_view;
use super::gesture::{
    EDGE_SWIPE_BLOCKING_SELECTOR, SwipeOutcome, SwipeSession, SwipeStartContext, SwipeTarget,
    SystemGestureInsets, TouchPoint, clear_swipe_session, edge_activation_bands,
    normalize_native_gesture_insets, resolve_swipe_outcome, resolve_swipe_start,
    resolve_swipe_start_for_surface, resolve_touch_end_outcome,
};
use crate::components::activity_bar::SidebarView;

fn point(x: i32, y: i32) -> TouchPoint {
    TouchPoint { x, y }
}

fn work_edit_context(system_gesture_insets: Option<SystemGestureInsets>) -> SwipeStartContext {
    SwipeStartContext {
        width: 500,
        show_sidebar: false,
        show_outline: false,
        interactive_target: false,
        work_edit_surface: true,
        touch_count: 1,
        system_gesture_insets,
    }
}

fn non_work_edit_context() -> SwipeStartContext {
    SwipeStartContext {
        work_edit_surface: false,
        ..work_edit_context(Some(SystemGestureInsets::web_default()))
    }
}

#[test]
fn mobile_drawer_edge_swipe_opens_left_from_left_edge() {
    let session = resolve_swipe_start(
        point(10, 200),
        500,
        false,
        false,
        false,
        1,
        Some(SystemGestureInsets::web_default()),
    );
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
    let session = resolve_swipe_start(
        point(490, 200),
        500,
        false,
        false,
        false,
        1,
        Some(SystemGestureInsets::web_default()),
    );
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
fn mobile_drawer_edge_swipe_work_edit_center_opens_both_drawers_by_direction() {
    let rightward = resolve_swipe_start_for_surface(
        point(250, 200),
        work_edit_context(Some(SystemGestureInsets::web_default())),
    );
    assert_eq!(
        resolve_swipe_outcome(rightward, point(330, 205)),
        SwipeOutcome::OpenLeft
    );

    let leftward = resolve_swipe_start_for_surface(
        point(250, 200),
        work_edit_context(Some(SystemGestureInsets::web_default())),
    );
    assert_eq!(
        resolve_swipe_outcome(leftward, point(170, 195)),
        SwipeOutcome::OpenRight
    );
}

#[test]
fn mobile_drawer_edge_swipe_work_edit_keeps_edge_bands_directional() {
    let left_band = resolve_swipe_start_for_surface(
        point(10, 200),
        work_edit_context(Some(SystemGestureInsets::web_default())),
    );
    assert_eq!(
        left_band.map(|session| session.target),
        Some(SwipeTarget::OpenLeft)
    );
    assert_eq!(
        resolve_swipe_outcome(left_band, point(-70, 200)),
        SwipeOutcome::None
    );

    let right_band = resolve_swipe_start_for_surface(
        point(490, 200),
        work_edit_context(Some(SystemGestureInsets::web_default())),
    );
    assert_eq!(
        right_band.map(|session| session.target),
        Some(SwipeTarget::OpenRight)
    );
    assert_eq!(
        resolve_swipe_outcome(right_band, point(570, 200)),
        SwipeOutcome::None
    );
}

#[test]
fn mobile_drawer_edge_swipe_work_edit_rejects_system_region_and_missing_presentation_hint() {
    let native = normalize_native_gesture_insets(7, 1000.0, 48.0, 48.0, 2.0, 500)
        .expect("valid native gesture insets");
    assert_eq!(
        resolve_swipe_start_for_surface(point(20, 200), work_edit_context(Some(native))),
        None
    );
    assert_eq!(
        resolve_swipe_start_for_surface(point(250, 200), work_edit_context(None)),
        None
    );
}

#[test]
fn mobile_drawer_edge_swipe_center_is_rejected_outside_work_edit() {
    assert_eq!(
        resolve_swipe_start_for_surface(point(250, 200), non_work_edit_context()),
        None
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
    let web = Some(SystemGestureInsets::web_default());
    assert!(resolve_swipe_start(point(20, 200), 500, false, false, false, 1, web).is_some());
    assert!(resolve_swipe_start(point(21, 200), 500, false, false, false, 1, web).is_none());
    assert!(resolve_swipe_start(point(480, 200), 500, false, false, false, 1, web).is_some());
    assert!(resolve_swipe_start(point(479, 200), 500, false, false, false, 1, web).is_none());

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
    let session = resolve_swipe_start(
        point(490, 200),
        500,
        false,
        false,
        true,
        1,
        Some(SystemGestureInsets::web_default()),
    );
    assert_eq!(session, None);
}

#[test]
fn mobile_drawer_edge_swipe_still_opens_outline_on_bare_edge_drag() {
    let session = resolve_swipe_start(
        point(490, 200),
        500,
        false,
        false,
        false,
        1,
        Some(SystemGestureInsets::web_default()),
    );
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

    let session = resolve_swipe_start(
        point(10, 200),
        500,
        false,
        false,
        false,
        1,
        Some(SystemGestureInsets::web_default()),
    );
    assert_eq!(
        session.map(|value| value.target),
        Some(SwipeTarget::OpenLeft)
    );
    let session = resolve_swipe_start(
        point(490, 200),
        500,
        false,
        false,
        false,
        1,
        Some(SystemGestureInsets::web_default()),
    );
    assert_eq!(
        session.map(|value| value.target),
        Some(SwipeTarget::OpenRight)
    );
}

#[test]
fn mobile_drawer_edge_swipe_rejects_multitouch_start() {
    let session = resolve_swipe_start(
        point(10, 200),
        500,
        false,
        false,
        false,
        2,
        Some(SystemGestureInsets::web_default()),
    );
    assert_eq!(session, None);
}

#[test]
fn mobile_drawer_edge_swipe_native_system_gesture_insets_shift_activation_bands_inward() {
    let native = normalize_native_gesture_insets(7, 1080.0, 62.0, 62.0, 2.75, 393)
        .expect("valid Xiaomi gesture insets");
    let bands = edge_activation_bands(393, Some(native)).expect("activation bands");

    assert_eq!(bands.left_start, 25);
    assert_eq!(bands.left_end, 45);
    assert_eq!(bands.right_start, 348);
    assert_eq!(bands.right_end, 368);
    assert!(
        resolve_swipe_start(point(29, 200), 393, false, false, false, 1, Some(native)).is_some()
    );
    assert!(
        resolve_swipe_start(point(370, 200), 393, false, false, false, 1, Some(native)).is_none()
    );
}

#[test]
fn mobile_drawer_edge_swipe_native_zero_reported_insets_use_safe_floor() {
    let native = normalize_native_gesture_insets(8, 1080.0, 0.0, 0.0, 2.75, 393)
        .expect("OEM zero insets remain a valid presentation fact");
    let bands = edge_activation_bands(393, Some(native)).expect("safe-floor activation bands");

    assert_eq!(bands.left_start, 25);
    assert_eq!(bands.left_end, 45);
    assert_eq!(bands.right_start, 348);
    assert_eq!(bands.right_end, 368);
    assert!(
        resolve_swipe_start(point(30, 200), 393, false, false, false, 1, Some(native),).is_some()
    );
    assert!(
        resolve_swipe_start(point(20, 200), 393, false, false, false, 1, Some(native),).is_none()
    );
}

#[test]
fn mobile_drawer_edge_swipe_native_system_gesture_insets_fail_closed_until_valid_hint() {
    assert!(resolve_swipe_start(point(10, 200), 393, false, false, false, 1, None).is_none());
    assert!(normalize_native_gesture_insets(0, 1080.0, 62.0, 62.0, 2.75, 393).is_none());
    assert!(normalize_native_gesture_insets(1, 1080.0, 600.0, 600.0, 2.75, 393).is_none());
    assert!(normalize_native_gesture_insets(1, 1080.0, 62.0, 62.0, 1.0, 393).is_none());
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

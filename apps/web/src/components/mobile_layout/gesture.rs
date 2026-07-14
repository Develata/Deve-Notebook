// apps/web/src/components/mobile_layout/gesture.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::TouchEvent;

const EDGE_ZONE: i32 = 20;
const SWIPE_THRESHOLD: i32 = 50;
pub(super) const EDGE_SWIPE_BLOCKING_SELECTOR: &str =
    "button, a, input, textarea, select, summary, [role='button'], [data-no-edge-swipe]";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TouchPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwipeTarget {
    OpenLeft,
    OpenRight,
    CloseLeft,
    CloseRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SwipeSession {
    pub target: SwipeTarget,
    start: TouchPoint,
}

impl SwipeSession {
    pub(super) fn new(target: SwipeTarget, start: TouchPoint) -> Self {
        Self { target, start }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SwipeOutcome {
    OpenLeft,
    OpenRight,
    CloseLeft,
    CloseRight,
    None,
}

pub fn build_touch_start(
    show_sidebar: ReadSignal<bool>,
    show_outline: ReadSignal<bool>,
    set_swipe_session: WriteSignal<Option<SwipeSession>>,
) -> Callback<TouchEvent> {
    Callback::new(move |ev: TouchEvent| {
        let Some(start) = first_changed_touch_point(&ev) else {
            set_swipe_session.set(None);
            return;
        };
        let width = window_width().unwrap_or(0);
        let session = resolve_swipe_start(
            start,
            width,
            show_sidebar.get_untracked(),
            show_outline.get_untracked(),
            is_interactive_target(&ev),
            ev.touches().length(),
        );
        set_swipe_session.set(session);
    })
}

pub fn build_touch_end(
    swipe_session: ReadSignal<Option<SwipeSession>>,
    open_left_drawer: Callback<()>,
    open_right_drawer: Callback<()>,
    close_left_drawer: Callback<()>,
    close_right_drawer: Callback<()>,
    set_swipe_session: WriteSignal<Option<SwipeSession>>,
) -> Callback<TouchEvent> {
    Callback::new(move |ev: TouchEvent| {
        let (outcome, next_session) = resolve_touch_end_outcome(
            swipe_session.get_untracked(),
            first_changed_touch_point(&ev),
            ev.touches().length(),
        );
        match outcome {
            SwipeOutcome::OpenLeft => open_left_drawer.run(()),
            SwipeOutcome::OpenRight => open_right_drawer.run(()),
            SwipeOutcome::CloseLeft => close_left_drawer.run(()),
            SwipeOutcome::CloseRight => close_right_drawer.run(()),
            SwipeOutcome::None => {}
        }
        set_swipe_session.set(next_session);
    })
}

fn first_changed_touch_point(ev: &TouchEvent) -> Option<TouchPoint> {
    let touches = ev.changed_touches();
    let touch = touches.get(0)?;
    Some(TouchPoint {
        x: touch.client_x(),
        y: touch.client_y(),
    })
}

pub(super) fn resolve_swipe_start(
    start: TouchPoint,
    width: i32,
    show_sidebar: bool,
    show_outline: bool,
    interactive_target: bool,
    touch_count: u32,
) -> Option<SwipeSession> {
    if interactive_target || touch_count != 1 {
        return None;
    }
    let target = if show_sidebar {
        Some(SwipeTarget::CloseLeft)
    } else if show_outline {
        Some(SwipeTarget::CloseRight)
    } else if start.x <= EDGE_ZONE {
        Some(SwipeTarget::OpenLeft)
    } else if width > 0 && start.x >= width - EDGE_ZONE {
        Some(SwipeTarget::OpenRight)
    } else {
        None
    };
    target.map(|target| SwipeSession::new(target, start))
}

pub(super) fn resolve_swipe_outcome(
    session: Option<SwipeSession>,
    end: TouchPoint,
) -> SwipeOutcome {
    let Some(session) = session else {
        return SwipeOutcome::None;
    };
    let delta_x = end.x - session.start.x;
    let delta_y = end.y - session.start.y;
    if delta_x.unsigned_abs() < SWIPE_THRESHOLD as u32
        || delta_x.unsigned_abs() <= delta_y.unsigned_abs()
    {
        return SwipeOutcome::None;
    }
    match session.target {
        SwipeTarget::OpenLeft if delta_x > 0 => SwipeOutcome::OpenLeft,
        SwipeTarget::OpenRight if delta_x < 0 => SwipeOutcome::OpenRight,
        SwipeTarget::CloseLeft if delta_x < 0 => SwipeOutcome::CloseLeft,
        SwipeTarget::CloseRight if delta_x > 0 => SwipeOutcome::CloseRight,
        _ => SwipeOutcome::None,
    }
}

pub(super) fn clear_swipe_session(session: &mut Option<SwipeSession>) {
    *session = None;
}

pub(super) fn resolve_touch_end_outcome(
    session: Option<SwipeSession>,
    end: Option<TouchPoint>,
    remaining_touches: u32,
) -> (SwipeOutcome, Option<SwipeSession>) {
    if remaining_touches != 0 {
        return (SwipeOutcome::None, None);
    }
    let Some(end) = end else {
        return (SwipeOutcome::None, None);
    };
    (resolve_swipe_outcome(session, end), None)
}

fn is_interactive_target(ev: &TouchEvent) -> bool {
    let Some(target) = ev.target() else {
        return false;
    };
    let element = target.dyn_ref::<web_sys::Element>().cloned().or_else(|| {
        target
            .dyn_ref::<web_sys::Node>()
            .and_then(|node| node.parent_element())
    });
    let Some(element) = element else {
        return false;
    };
    element
        .closest(EDGE_SWIPE_BLOCKING_SELECTOR)
        .ok()
        .flatten()
        .is_some()
}

pub fn window_width() -> Option<i32> {
    let window = web_sys::window()?;
    let width = window.inner_width().ok()?.as_f64()?;
    Some(width as i32)
}

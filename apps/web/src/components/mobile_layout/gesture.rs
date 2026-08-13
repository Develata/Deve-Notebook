// apps/web/src/components/mobile_layout/gesture.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::TouchEvent;

const APP_EDGE_ZONE: i32 = 20;
const ANDROID_SYSTEM_GESTURE_SAFE_FLOOR_CSS: i32 = 24;
const SWIPE_THRESHOLD: i32 = 50;
pub(super) const EDGE_SWIPE_BLOCKING_SELECTOR: &str =
    "button, a, input, textarea, select, summary, [role='button'], [data-no-edge-swipe]";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TouchPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SystemGestureInsets {
    generation: u64,
    left_css: i32,
    right_css: i32,
}

impl SystemGestureInsets {
    pub(super) const fn web_default() -> Self {
        Self {
            generation: 0,
            left_css: 0,
            right_css: 0,
        }
    }

    pub(super) fn is_native(self) -> bool {
        self.generation > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EdgeActivationBands {
    pub(super) left_start: i32,
    pub(super) left_end: i32,
    pub(super) right_start: i32,
    pub(super) right_end: i32,
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
    system_gesture_insets: ReadSignal<Option<SystemGestureInsets>>,
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
            system_gesture_insets.get_untracked(),
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
    system_gesture_insets: Option<SystemGestureInsets>,
) -> Option<SwipeSession> {
    if interactive_target || touch_count != 1 {
        return None;
    }
    let target = if show_sidebar {
        Some(SwipeTarget::CloseLeft)
    } else if show_outline {
        Some(SwipeTarget::CloseRight)
    } else {
        let bands = edge_activation_bands(width, system_gesture_insets)?;
        if (bands.left_start..=bands.left_end).contains(&start.x) {
            Some(SwipeTarget::OpenLeft)
        } else if (bands.right_start..=bands.right_end).contains(&start.x) {
            Some(SwipeTarget::OpenRight)
        } else {
            None
        }
    };
    target.map(|target| SwipeSession::new(target, start))
}

pub(super) fn normalize_native_gesture_insets(
    generation: u64,
    width_px: f64,
    left_px: f64,
    right_px: f64,
    density: f64,
    viewport_width_css: i32,
) -> Option<SystemGestureInsets> {
    if generation == 0
        || viewport_width_css <= 0
        || !width_px.is_finite()
        || !left_px.is_finite()
        || !right_px.is_finite()
        || !density.is_finite()
        || width_px <= 0.0
        || left_px < 0.0
        || right_px < 0.0
        || !(0.5..=8.0).contains(&density)
        || left_px + right_px >= width_px
    {
        return None;
    }
    let projected_width = width_px / density;
    if (projected_width - f64::from(viewport_width_css)).abs() > 2.0 {
        return None;
    }
    let left_css = ((left_px / density).ceil() as i32).max(ANDROID_SYSTEM_GESTURE_SAFE_FLOOR_CSS);
    let right_css = ((right_px / density).ceil() as i32).max(ANDROID_SYSTEM_GESTURE_SAFE_FLOOR_CSS);
    let insets = SystemGestureInsets {
        generation,
        left_css,
        right_css,
    };
    edge_activation_bands(viewport_width_css, Some(insets))?;
    Some(insets)
}

pub(super) fn edge_activation_bands(
    width: i32,
    system_gesture_insets: Option<SystemGestureInsets>,
) -> Option<EdgeActivationBands> {
    let insets = system_gesture_insets?;
    if width <= 0 || insets.left_css < 0 || insets.right_css < 0 {
        return None;
    }
    let native_guard = i32::from(insets.generation > 0);
    let left_start = insets.left_css.checked_add(native_guard)?;
    let left_end = left_start.checked_add(APP_EDGE_ZONE)?;
    let right_end = width
        .checked_sub(insets.right_css)?
        .checked_sub(native_guard)?;
    let right_start = right_end.checked_sub(APP_EDGE_ZONE)?;
    if left_start < 0
        || right_end > width
        || left_end >= right_start
        || right_start < 0
        || right_end <= 0
    {
        return None;
    }
    Some(EdgeActivationBands {
        left_start,
        left_end,
        right_start,
        right_end,
    })
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

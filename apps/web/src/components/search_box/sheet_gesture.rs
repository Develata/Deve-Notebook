use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, TouchEvent};

const CLOSE_THRESHOLD_PX: i32 = 72;
const MIN_DRAG_PX: i32 = 12;
const QUICK_TAP_MS: f64 = 90.0;
const QUICK_TAP_MAX_DRAG_PX: i32 = 20;
const FAST_FLICK_PX_PER_MS: f64 = 0.9;
const FAST_FLICK_MIN_DRAG_PX: i32 = 24;

pub fn on_start(
    ev: &TouchEvent,
    results_ref: &NodeRef<leptos::html::Div>,
    set_touch_start_x: WriteSignal<i32>,
    set_touch_start_y: WriteSignal<i32>,
    set_touch_start_at: WriteSignal<f64>,
    set_can_dismiss: WriteSignal<bool>,
) {
    let (x, y) = match first_touch_xy(ev) {
        Some(v) => v,
        None => return,
    };
    set_touch_start_x.set(x);
    set_touch_start_y.set(y);
    set_touch_start_at.set(now_ms());
    set_can_dismiss.set(can_start_dismiss(ev, results_ref));
}

pub fn should_close(
    ev: &TouchEvent,
    touch_start_x: ReadSignal<i32>,
    touch_start_y: ReadSignal<i32>,
    touch_start_at: ReadSignal<f64>,
    can_dismiss: ReadSignal<bool>,
) -> bool {
    if !can_dismiss.get_untracked() {
        return false;
    }
    let (end_x, end_y) = match first_touch_xy(ev) {
        Some(v) => v,
        None => return false,
    };
    let delta_x = (end_x - touch_start_x.get_untracked()).abs();
    let delta_y = end_y - touch_start_y.get_untracked();
    let upward_drag = -delta_y;
    if upward_drag <= MIN_DRAG_PX {
        return false;
    }
    if delta_x > upward_drag {
        return false;
    }
    let elapsed = now_ms() - touch_start_at.get_untracked();
    if elapsed <= QUICK_TAP_MS && upward_drag <= QUICK_TAP_MAX_DRAG_PX {
        return false;
    }
    if elapsed > 0.0
        && upward_drag >= FAST_FLICK_MIN_DRAG_PX
        && (upward_drag as f64 / elapsed) >= FAST_FLICK_PX_PER_MS
    {
        return true;
    }
    upward_drag >= CLOSE_THRESHOLD_PX
}

pub fn reset(set_can_dismiss: WriteSignal<bool>) {
    set_can_dismiss.set(false);
}

pub fn damped_offset(start_y: i32, current_y: i32, can_dismiss: bool) -> i32 {
    if !can_dismiss {
        return 0;
    }
    let delta = current_y - start_y;
    if delta >= 0 {
        return 0;
    }
    let upward = (-delta) as f32;
    let damped = if upward <= 60.0 {
        upward * 0.62
    } else {
        37.2 + (upward - 60.0).sqrt() * 6.2
    };
    -(damped.min(120.0) as i32)
}

fn can_start_dismiss(ev: &TouchEvent, results_ref: &NodeRef<leptos::html::Div>) -> bool {
    if results_ref.get_untracked().is_none() {
        return true;
    }
    let Some(target_element) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
        return true;
    };
    let inside_results = target_element
        .closest("[data-sheet-results='1']")
        .ok()
        .flatten()
        .is_some();
    if inside_results {
        return false;
    }
    target_element
        .closest("[data-sheet-drag-handle='1']")
        .ok()
        .flatten()
        .is_some()
}

fn first_touch_xy(ev: &TouchEvent) -> Option<(i32, i32)> {
    let touch = ev.changed_touches().get(0)?;
    Some((touch.client_x(), touch.client_y()))
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

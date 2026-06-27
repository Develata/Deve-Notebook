// apps/web/src/components/mobile_layout/gesture.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::TouchEvent;

const EDGE_ZONE: i32 = 20;
const SWIPE_THRESHOLD: i32 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwipeTarget {
    OpenLeft,
    OpenRight,
    CloseLeft,
    CloseRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SwipeOutcome {
    OpenLeft,
    OpenRight,
    CloseDrawers,
    None,
}

pub fn build_touch_start(
    show_sidebar: ReadSignal<bool>,
    show_outline: ReadSignal<bool>,
    set_swipe_start_x: WriteSignal<i32>,
    set_swipe_target: WriteSignal<Option<SwipeTarget>>,
) -> Callback<TouchEvent> {
    Callback::new(move |ev: TouchEvent| {
        let x = match first_touch_x(&ev) {
            Some(v) => v,
            None => return,
        };
        let width = window_width().unwrap_or(0);
        let target = resolve_swipe_target(
            x,
            width,
            show_sidebar.get_untracked(),
            show_outline.get_untracked(),
            is_interactive_target(&ev),
        );
        set_swipe_start_x.set(x);
        set_swipe_target.set(target);
    })
}

pub fn build_touch_end(
    swipe_target: ReadSignal<Option<SwipeTarget>>,
    swipe_start_x: ReadSignal<i32>,
    set_show_sidebar: WriteSignal<bool>,
    set_show_outline: WriteSignal<bool>,
    close_drawers: Callback<()>,
    set_swipe_target: WriteSignal<Option<SwipeTarget>>,
) -> Callback<TouchEvent> {
    Callback::new(move |ev: TouchEvent| {
        let target = swipe_target.get_untracked();
        let start_x = swipe_start_x.get_untracked();
        let (outcome, next_target) = resolve_touch_end_outcome(target, start_x, first_touch_x(&ev));
        match outcome {
            SwipeOutcome::OpenLeft => {
                set_show_sidebar.set(true);
                set_show_outline.set(false);
            }
            SwipeOutcome::OpenRight => {
                set_show_outline.set(true);
                set_show_sidebar.set(false);
            }
            SwipeOutcome::CloseDrawers => close_drawers.run(()),
            SwipeOutcome::None => {}
        }
        set_swipe_target.set(next_target);
    })
}

fn first_touch_x(ev: &TouchEvent) -> Option<i32> {
    let touches = ev.changed_touches();
    let touch = touches.get(0)?;
    Some(touch.client_x())
}

pub(super) fn resolve_swipe_target(
    x: i32,
    width: i32,
    show_sidebar: bool,
    show_outline: bool,
    interactive_target: bool,
) -> Option<SwipeTarget> {
    if interactive_target {
        return None;
    }
    if show_sidebar {
        Some(SwipeTarget::CloseLeft)
    } else if show_outline {
        Some(SwipeTarget::CloseRight)
    } else if x <= EDGE_ZONE {
        Some(SwipeTarget::OpenLeft)
    } else if width > 0 && x >= width - EDGE_ZONE {
        Some(SwipeTarget::OpenRight)
    } else {
        None
    }
}

pub(super) fn resolve_swipe_outcome(target: Option<SwipeTarget>, delta: i32) -> SwipeOutcome {
    match target {
        Some(SwipeTarget::OpenLeft) if delta >= SWIPE_THRESHOLD => SwipeOutcome::OpenLeft,
        Some(SwipeTarget::OpenRight) if delta <= -SWIPE_THRESHOLD => SwipeOutcome::OpenRight,
        Some(SwipeTarget::CloseLeft) if delta <= -SWIPE_THRESHOLD => SwipeOutcome::CloseDrawers,
        Some(SwipeTarget::CloseRight) if delta >= SWIPE_THRESHOLD => SwipeOutcome::CloseDrawers,
        _ => SwipeOutcome::None,
    }
}

pub(super) fn resolve_touch_end_outcome(
    target: Option<SwipeTarget>,
    start_x: i32,
    end_x: Option<i32>,
) -> (SwipeOutcome, Option<SwipeTarget>) {
    let Some(end_x) = end_x else {
        return (SwipeOutcome::None, None);
    };
    (resolve_swipe_outcome(target, end_x - start_x), None)
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
        .closest(
            "button, a, input, textarea, select, summary, [role='button'], \
             [contenteditable='true'], [data-no-edge-swipe]",
        )
        .ok()
        .flatten()
        .is_some()
}

pub fn window_width() -> Option<i32> {
    let window = web_sys::window()?;
    let width = window.inner_width().ok()?.as_f64()?;
    Some(width as i32)
}

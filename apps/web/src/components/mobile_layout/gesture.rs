// apps/web/src/components/mobile_layout/gesture.rs

use leptos::prelude::*;
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
        let target = if show_sidebar.get_untracked() {
            Some(SwipeTarget::CloseLeft)
        } else if show_outline.get_untracked() {
            Some(SwipeTarget::CloseRight)
        } else if x <= EDGE_ZONE {
            Some(SwipeTarget::OpenLeft)
        } else if width > 0 && x >= width - EDGE_ZONE {
            Some(SwipeTarget::OpenRight)
        } else {
            None
        };
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
        let end_x = match first_touch_x(&ev) {
            Some(v) => v,
            None => return,
        };
        let delta = end_x - start_x;
        match target {
            Some(SwipeTarget::OpenLeft) if delta >= SWIPE_THRESHOLD => {
                set_show_sidebar.set(true);
                set_show_outline.set(false);
            }
            Some(SwipeTarget::OpenRight) if delta <= -SWIPE_THRESHOLD => {
                set_show_outline.set(true);
                set_show_sidebar.set(false);
            }
            Some(SwipeTarget::CloseLeft) if delta <= -SWIPE_THRESHOLD => close_drawers.run(()),
            Some(SwipeTarget::CloseRight) if delta >= SWIPE_THRESHOLD => close_drawers.run(()),
            _ => {}
        }
        set_swipe_target.set(None);
    })
}

fn first_touch_x(ev: &TouchEvent) -> Option<i32> {
    let touches = ev.changed_touches();
    let touch = touches.get(0)?;
    Some(touch.client_x())
}

pub fn window_width() -> Option<i32> {
    let window = web_sys::window()?;
    let width = window.inner_width().ok()?.as_f64()?;
    Some(width as i32)
}

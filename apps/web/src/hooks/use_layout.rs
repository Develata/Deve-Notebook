// apps/web/src/hooks/use_layout.rs
//! # Layout Hook (布局钩子)
//!
//! **架构作用**:
//! 管理可调整大小的侧边栏与右侧面板布局状态。
//!
//! **核心功能清单**:
//! - `use_layout`: 返回左右面板宽度信号和拖拽处理回调。

use leptos::prelude::*;
use web_sys::PointerEvent;

/// 布局钩子返回类型
pub type LayoutHookReturn = (
    ReadSignal<i32>,
    ReadSignal<i32>,
    ReadSignal<i32>,
    Callback<PointerEvent>, // 开始拖拽 (左)
    Callback<PointerEvent>, // 开始拖拽 (右)
    Callback<PointerEvent>, // 开始拖拽 (外层左)
    Callback<PointerEvent>, // 开始拖拽 (外层右)
    Callback<()>,           // 停止拖拽
    Callback<PointerEvent>, // 执行拖拽
    ReadSignal<bool>,       // 是否正在拖拽
);

pub fn use_layout() -> LayoutHookReturn {
    const SIDEBAR_MIN: i32 = 180;
    const SIDEBAR_MAX: i32 = 500;
    const RIGHT_MIN: i32 = 240;
    const RIGHT_MAX: i32 = 520;
    const OUTER_MIN: i32 = 0;
    const OUTER_MAX: i32 = 120;

    let sidebar_initial = read_width("ui_sidebar_width").unwrap_or(250);
    let right_initial = read_width("ui_right_panel_width").unwrap_or(350);
    let outer_initial = read_width("ui_outer_gutter").unwrap_or(16);

    let (sidebar_width, set_sidebar_width) =
        signal(clamp(sidebar_initial, SIDEBAR_MIN, SIDEBAR_MAX));
    let (right_width, set_right_width) = signal(clamp(right_initial, RIGHT_MIN, RIGHT_MAX));
    let (outer_gutter, set_outer_gutter) = signal(clamp(outer_initial, OUTER_MIN, OUTER_MAX));
    let (is_resizing, set_is_resizing) = signal(false);
    let (active_resize, set_active_resize) = signal(None::<ResizeTarget>);
    let (start_x, set_start_x) = signal(0i32);
    let (start_width, set_start_width) = signal(0i32);

    Effect::new(move |_| {
        write_width("ui_sidebar_width", sidebar_width.get());
    });

    Effect::new(move |_| {
        write_width("ui_right_panel_width", right_width.get());
    });

    Effect::new(move |_| {
        write_width("ui_outer_gutter", outer_gutter.get());
    });

    let start_resize_left = Callback::new(move |ev: PointerEvent| {
        ev.prevent_default();
        set_is_resizing.set(true);
        set_active_resize.set(Some(ResizeTarget::Left));
        set_start_x.set(ev.client_x());
        set_start_width.set(sidebar_width.get_untracked());
    });

    let start_resize_right = Callback::new(move |ev: PointerEvent| {
        ev.prevent_default();
        set_is_resizing.set(true);
        set_active_resize.set(Some(ResizeTarget::Right));
        set_start_x.set(ev.client_x());
        set_start_width.set(right_width.get_untracked());
    });

    let start_resize_outer_left = Callback::new(move |ev: PointerEvent| {
        ev.prevent_default();
        set_is_resizing.set(true);
        set_active_resize.set(Some(ResizeTarget::OuterLeft));
        set_start_x.set(ev.client_x());
        set_start_width.set(outer_gutter.get_untracked());
    });

    let start_resize_outer_right = Callback::new(move |ev: PointerEvent| {
        ev.prevent_default();
        set_is_resizing.set(true);
        set_active_resize.set(Some(ResizeTarget::OuterRight));
        set_start_x.set(ev.client_x());
        set_start_width.set(outer_gutter.get_untracked());
    });

    let stop_resize = Callback::new(move |_| {
        set_is_resizing.set(false);
        set_active_resize.set(None);
    });

    let do_resize = Callback::new(move |ev: PointerEvent| {
        if !is_resizing.get_untracked() {
            return;
        }
        let delta = ev.client_x() - start_x.get_untracked();
        match active_resize.get_untracked() {
            Some(ResizeTarget::Left) => {
                let width = start_width.get_untracked() + delta;
                set_sidebar_width.set(clamp(width, SIDEBAR_MIN, SIDEBAR_MAX));
            }
            Some(ResizeTarget::Right) => {
                let width = start_width.get_untracked() - delta;
                set_right_width.set(clamp(width, RIGHT_MIN, RIGHT_MAX));
            }
            Some(ResizeTarget::OuterLeft) => {
                let width = start_width.get_untracked() + delta;
                set_outer_gutter.set(clamp(width, OUTER_MIN, OUTER_MAX));
            }
            Some(ResizeTarget::OuterRight) => {
                let width = start_width.get_untracked() - delta;
                set_outer_gutter.set(clamp(width, OUTER_MIN, OUTER_MAX));
            }
            None => {}
        }
    });

    (
        sidebar_width,
        right_width,
        outer_gutter,
        start_resize_left,
        start_resize_right,
        start_resize_outer_left,
        start_resize_outer_right,
        stop_resize,
        do_resize,
        is_resizing,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResizeTarget {
    Left,
    Right,
    OuterLeft,
    OuterRight,
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    value.clamp(min, max)
}

fn read_width(key: &str) -> Option<i32> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let val = storage.get_item(key).ok()??;
    val.parse::<i32>().ok()
}

fn write_width(key: &str, value: i32) {
    if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) {
        let _ = storage.set_item(key, &value.to_string());
    }
}

// apps/web/src/hooks/use_layout.rs
//! Layout Hook for resizable sidebar, chat panel, and outer gutter widths.

use self::use_layout_resize::{
    ResizeTarget, do_resize_callback, start_resize_callback, stop_resize_callback,
};
use self::use_layout_storage::{clamp, read_width, write_width};
use leptos::prelude::*;
use web_sys::PointerEvent;

#[path = "use_layout_resize.rs"]
mod use_layout_resize;
#[path = "use_layout_storage.rs"]
mod use_layout_storage;

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
    let (active_pointer, set_active_pointer) = signal(None::<i32>);

    Effect::new(move |_| {
        write_width("ui_sidebar_width", sidebar_width.get());
    });

    Effect::new(move |_| {
        write_width("ui_right_panel_width", right_width.get());
    });

    Effect::new(move |_| {
        write_width("ui_outer_gutter", outer_gutter.get());
    });

    let start_resize_left = start_resize_callback(
        set_is_resizing,
        set_active_resize,
        set_start_x,
        set_start_width,
        set_active_pointer,
        sidebar_width,
        ResizeTarget::Left,
    );
    let start_resize_right = start_resize_callback(
        set_is_resizing,
        set_active_resize,
        set_start_x,
        set_start_width,
        set_active_pointer,
        right_width,
        ResizeTarget::Right,
    );
    let start_resize_outer_left = start_resize_callback(
        set_is_resizing,
        set_active_resize,
        set_start_x,
        set_start_width,
        set_active_pointer,
        outer_gutter,
        ResizeTarget::OuterLeft,
    );
    let start_resize_outer_right = start_resize_callback(
        set_is_resizing,
        set_active_resize,
        set_start_x,
        set_start_width,
        set_active_pointer,
        outer_gutter,
        ResizeTarget::OuterRight,
    );
    let stop_resize = stop_resize_callback(set_is_resizing, set_active_resize, set_active_pointer);
    let do_resize = do_resize_callback(
        is_resizing,
        active_pointer,
        start_x,
        start_width,
        active_resize,
        set_sidebar_width,
        set_right_width,
        set_outer_gutter,
        (SIDEBAR_MIN, SIDEBAR_MAX),
        (RIGHT_MIN, RIGHT_MAX),
        (OUTER_MIN, OUTER_MAX),
    );

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

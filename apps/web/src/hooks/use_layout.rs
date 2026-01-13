//! # Layout Hook (布局钩子)
//!
//! **架构作用**:
//! 管理可调整大小的侧边栏布局状态。
//!
//! **核心功能清单**:
//! - `use_layout`: 返回侧边栏宽度信号和拖拽处理回调。

use leptos::prelude::*;
use web_sys::MouseEvent;

pub fn use_layout() -> (
    ReadSignal<i32>,
    Callback<MouseEvent>, // 开始拖拽
    Callback<()>,         // 停止拖拽
    Callback<MouseEvent>, // 执行拖拽
    ReadSignal<bool>      // 是否正在拖拽
) {
    let (sidebar_width, set_sidebar_width) = signal(250);
    let (is_resizing, set_is_resizing) = signal(false);
    
    let start_resize = Callback::new(move |ev: MouseEvent| {
        ev.prevent_default();
        set_is_resizing.set(true);
    });
    
    let stop_resize = Callback::new(move |_| {
        set_is_resizing.set(false);
    });
    
    let do_resize = Callback::new(move |ev: MouseEvent| {
        if is_resizing.get_untracked() {
            let new_width = ev.client_x();
            // 限制宽度范围 (最小 150, 最大 600)
            if new_width > 150 && new_width < 600 {
                set_sidebar_width.set(new_width);
            }
        }
    });

    (sidebar_width, start_resize, stop_resize, do_resize, is_resizing)
}

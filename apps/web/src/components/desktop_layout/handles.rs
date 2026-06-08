//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use leptos::prelude::*;
use wasm_bindgen::JsCast;

fn capture_pointer(ev: &web_sys::PointerEvent) {
    if let Some(target) = ev.target()
        && let Ok(el) = target.dyn_into::<web_sys::Element>()
    {
        let _ = el.set_pointer_capture(ev.pointer_id());
    }
}

#[component]
pub fn DesktopOuterResizeHandle(
    side: &'static str,
    outer_gutter: Signal<i32>,
    on_resize: Callback<web_sys::PointerEvent>,
) -> impl IntoView {
    let transform = if side == "left" {
        "translateX(-50%)"
    } else {
        "translateX(50%)"
    };

    view! {
        <div
            data-deve-desktop-resizer=move || format!("outer-{side}")
            class="resizer-handle absolute top-0 h-full w-3 cursor-col-resize touch-none"
            style=move || {
                format!(
                    "{side}: {}px; transform: {transform};",
                    outer_gutter.get(),
                )
            }
            on:pointerdown=move |ev| {
                capture_pointer(&ev);
                on_resize.run(ev);
            }
        ></div>
    }
}

#[component]
pub fn DesktopInnerResizeHandle(
    marker: &'static str,
    on_resize: Callback<web_sys::PointerEvent>,
) -> impl IntoView {
    view! {
        <div
            data-deve-desktop-resizer=marker
            class="resizer-handle w-4 flex-none cursor-col-resize flex items-center justify-center hover:bg-accent-subtle group transition-colors touch-none"
            on:pointerdown=move |ev| {
                capture_pointer(&ev);
                on_resize.run(ev);
            }
        >
            <div class="w-[1px] h-8 bg-active group-hover:bg-accent transition-colors"></div>
        </div>
    }
}

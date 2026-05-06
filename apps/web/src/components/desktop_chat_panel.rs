// apps/web/src/components/desktop_chat_panel.rs
//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!   - 08_ui_design_01_web#web-layout-persistence
//!
//! # Desktop Chat Panel with Resize Gutter
//!
//! Conditionally rendered right-side chat panel for desktop layout.

use crate::components::chat::ChatPanel;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn DesktopChatPanel(
    chat_visible: ReadSignal<bool>,
    right_width: ReadSignal<i32>,
    start_resize_right: Callback<web_sys::PointerEvent>,
) -> impl IntoView {
    move || {
        if !chat_visible.get() {
            return view! {}.into_any();
        }
        view! {
            <div class="flex items-stretch ml-4">
                <div
                    class="resizer-handle w-4 flex-none cursor-col-resize flex items-center justify-center hover:bg-accent-subtle group transition-colors touch-none"
                    on:pointerdown=move |ev| {
                        if let Some(target) = ev.target()
                            && let Ok(el) = target.dyn_into::<web_sys::Element>()
                        {
                            let _ = el.set_pointer_capture(ev.pointer_id());
                        }
                        start_resize_right.run(ev)
                    }
                >
                    <div class="w-[1px] h-8 bg-active group-hover:bg-accent transition-colors"></div>
                </div>
                <div
                    data-deve-desktop-col="5-chat"
                    class="flex-none bg-panel shadow-sm border border-default rounded-lg overflow-hidden flex flex-col"
                    style=move || format!("width: {}px", right_width.get())
                >
                    <ChatPanel on_close=Callback::new(move |_| ()) />
                </div>
            </div>
        }
        .into_any()
    }
}

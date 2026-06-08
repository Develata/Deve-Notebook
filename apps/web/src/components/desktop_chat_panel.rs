// apps/web/src/components/desktop_chat_panel.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
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
    right_width: Signal<i32>,
    start_resize_right: Callback<web_sys::PointerEvent>,
) -> impl IntoView {
    move || {
        if !chat_visible.get() {
            return view! {}.into_any();
        }
        view! {
            <div class="contents" data-deve-desktop-chat-region="visible">
                <div
                    data-deve-desktop-resizer="right-divider"
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
                    data-deve-desktop-col-width=move || right_width.get().to_string()
                    aria-hidden=move || (right_width.get() == 0).to_string()
                    class="min-w-0 bg-panel shadow-sm border border-default rounded-lg overflow-hidden flex flex-col"
                    style=move || {
                        if right_width.get() == 0 {
                            "visibility: hidden; pointer-events: none;".to_string()
                        } else {
                            String::new()
                        }
                    }
                >
                    <ChatPanel on_close=Callback::new(move |_| ()) />
                </div>
            </div>
        }
        .into_any()
    }
}

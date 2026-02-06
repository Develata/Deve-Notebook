use crate::editor::ffi;
use leptos::prelude::*;

#[component]
pub fn MobileAccessoryToolbar(
    keyboard_offset: ReadSignal<i32>,
    readonly: Signal<bool>,
    visible: Signal<bool>,
) -> impl IntoView {
    const FOOTER_HEIGHT_PX: i32 = 0;
    let on_tab = Callback::new(move |_| {
        ffi::mobile_insert_text("\t");
    });
    let on_h1 = Callback::new(move |_| {
        ffi::mobile_insert_text("# ");
    });
    let on_list = Callback::new(move |_| {
        ffi::mobile_insert_text("- ");
    });
    let on_task = Callback::new(move |_| {
        ffi::mobile_insert_text("- [ ] ");
    });
    let on_bold = Callback::new(move |_| {
        ffi::mobile_wrap_selection("**", "**");
    });
    let on_italic = Callback::new(move |_| {
        ffi::mobile_wrap_selection("_", "_");
    });
    let on_code = Callback::new(move |_| {
        ffi::mobile_wrap_selection("`", "`");
    });
    let on_undo = Callback::new(move |_| {
        ffi::mobile_undo();
    });

    let base = "h-9 min-w-9 px-2 rounded-md border border-gray-200 bg-white text-gray-700 active:bg-gray-100 text-xs font-medium";
    let disabled = move || readonly.get();

    view! {
        <Show when=move || visible.get()>
            <div
                class="fixed left-0 right-0 z-50 bg-white/95 backdrop-blur border-t border-gray-200 px-2 py-2"
                style=move || {
                    format!(
                        "bottom: calc({}px + {}px + env(safe-area-inset-bottom));",
                        keyboard_offset.get(),
                        FOOTER_HEIGHT_PX
                    )
                }
            >
                <div class="flex items-center gap-1 overflow-x-auto">
                    <button class=base on:click=move |_| on_tab.run(()) disabled=disabled title="Tab">"⇥"</button>
                    <button class=base on:click=move |_| on_h1.run(()) disabled=disabled title="Heading">"H"</button>
                    <button class=base on:click=move |_| on_list.run(()) disabled=disabled title="List">"•"</button>
                    <button class=base on:click=move |_| on_task.run(()) disabled=disabled title="Task">"☑"</button>
                    <button class=base on:click=move |_| on_bold.run(()) disabled=disabled title="Bold">"B"</button>
                    <button class=base on:click=move |_| on_italic.run(()) disabled=disabled title="I">"I"</button>
                    <button class=base on:click=move |_| on_code.run(()) disabled=disabled title="Code">"<>"</button>
                    <button class=base on:click=move |_| on_undo.run(()) title="Undo">"↩"</button>
                </div>
            </div>
        </Show>
    }
}

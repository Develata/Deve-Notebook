//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-interaction-design
//!   - 03_rendering#large-document-runtime
//!
use crate::editor::ffi;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

const FOOTER_HEIGHT_PX: i32 = 0;

pub(super) fn mobile_toolbar_style(keyboard_offset: i32) -> String {
    format!(
        "bottom: calc({}px + {}px + env(safe-area-inset-bottom));",
        keyboard_offset, FOOTER_HEIGHT_PX
    )
}

#[component]
pub fn MobileAccessoryToolbar(
    keyboard_offset: ReadSignal<i32>,
    readonly: Signal<bool>,
    visible: Signal<bool>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
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

    let base = "h-9 min-w-9 px-2 rounded-md border border-default bg-panel text-primary active:bg-hover text-xs font-medium";
    let disabled = move || readonly.get();

    view! {
        <Show when=move || visible.get()>
            <div
                data-deve-mobile-toolbar="accessory"
                data-deve-keyboard-offset=move || keyboard_offset.get().to_string()
                class="fixed left-0 right-0 z-[var(--z-floating)] bg-panel/95 backdrop-blur border-t border-default px-2 py-2"
                style=move || mobile_toolbar_style(keyboard_offset.get())
            >
                <div class="flex items-center gap-1 overflow-x-auto">
                    <button class=base on:click=move |_| on_tab.run(()) disabled=disabled title=move || t::common::tab(locale.get())>"⇥"</button>
                    <button class=base on:click=move |_| on_h1.run(()) disabled=disabled title=move || t::common::heading(locale.get())>"H"</button>
                    <button class=base on:click=move |_| on_list.run(()) disabled=disabled title=move || t::common::list(locale.get())>"•"</button>
                    <button class=base on:click=move |_| on_task.run(()) disabled=disabled title=move || t::common::task(locale.get())>"☑"</button>
                    <button class=base on:click=move |_| on_bold.run(()) disabled=disabled title=move || t::common::bold(locale.get())>"B"</button>
                    <button class=base on:click=move |_| on_italic.run(()) disabled=disabled title=move || t::common::italic(locale.get())>"I"</button>
                    <button class=base on:click=move |_| on_code.run(()) disabled=disabled title=move || t::common::code(locale.get())>"<>"</button>
                    <button class=base on:click=move |_| on_undo.run(()) title=move || t::common::undo(locale.get())>"↩"</button>
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::mobile_toolbar_style;

    #[test]
    fn mobile_toolbar_keyboard_style_places_toolbar_above_keyboard() {
        assert_eq!(
            mobile_toolbar_style(312),
            "bottom: calc(312px + 0px + env(safe-area-inset-bottom));"
        );
    }
}

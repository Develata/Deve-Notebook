//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 10_rendering#large-document-runtime
//!
use crate::editor::ffi;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

const FOOTER_HEIGHT_PX: i32 = 0;
const TOOLBAR_HISTORY_ACTIONS: [&str; 2] = ["undo", "redo"];
const TOOLBAR_HISTORY_TARGET: &str = "mobile_toolbar_history_actions";

pub(super) fn mobile_toolbar_history_actions_front_order() -> [&'static str; 2] {
    TOOLBAR_HISTORY_ACTIONS
}

pub(super) fn toolbar_button_class() -> &'static str {
    "h-11 min-w-[44px] px-2 rounded-md border border-default bg-panel text-primary active:bg-hover text-xs font-medium disabled:opacity-50 disabled:cursor-not-allowed"
}

pub(super) fn toolbar_button_type() -> &'static str {
    "button"
}

pub(super) fn mobile_toolbar_style(keyboard_offset: i32) -> String {
    format!(
        "bottom: calc({}px + {}px + env(safe-area-inset-bottom));",
        keyboard_offset, FOOTER_HEIGHT_PX
    )
}

pub(super) fn mobile_toolbar_action_enabled(readonly: bool) -> bool {
    !readonly
}

fn run_mobile_toolbar_action(readonly: Signal<bool>, action: impl FnOnce()) {
    if mobile_toolbar_action_enabled(readonly.get_untracked()) {
        action();
    }
}

#[component]
pub fn MobileAccessoryToolbar(
    keyboard_offset: ReadSignal<i32>,
    readonly: Signal<bool>,
    visible: Signal<bool>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let on_tab = Callback::new(move |_| {
        run_mobile_toolbar_action(readonly, || ffi::mobile_insert_text("\t"));
    });
    let on_h1 = Callback::new(move |_| {
        run_mobile_toolbar_action(readonly, || ffi::mobile_insert_text("# "));
    });
    let on_list = Callback::new(move |_| {
        run_mobile_toolbar_action(readonly, || ffi::mobile_insert_text("- "));
    });
    let on_task = Callback::new(move |_| {
        run_mobile_toolbar_action(readonly, || ffi::mobile_insert_text("- [ ] "));
    });
    let on_bold = Callback::new(move |_| {
        run_mobile_toolbar_action(readonly, || ffi::mobile_wrap_selection("**", "**"));
    });
    let on_italic = Callback::new(move |_| {
        run_mobile_toolbar_action(readonly, || ffi::mobile_wrap_selection("_", "_"));
    });
    let on_code = Callback::new(move |_| {
        run_mobile_toolbar_action(readonly, || ffi::mobile_wrap_selection("`", "`"));
    });
    let on_undo = Callback::new(move |_| {
        run_mobile_toolbar_action(readonly, ffi::mobile_undo);
    });
    let on_redo = Callback::new(move |_| {
        run_mobile_toolbar_action(readonly, ffi::mobile_redo);
    });

    let base = toolbar_button_class();
    let button_type = toolbar_button_type();
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
                    <button
                        type=button_type
                        data-deve-mobile-touch-target="accessory_toolbar_buttons"
                        data-deve-mobile-toolbar-history-target=TOOLBAR_HISTORY_TARGET
                        data-deve-mobile-history-action=mobile_toolbar_history_actions_front_order()[0]
                        class=base
                        on:click=move |_| on_undo.run(())
                        disabled=disabled
                        title=move || t::common::undo(locale.get())
                        aria-label=move || t::common::undo(locale.get())
                    >
                        "↩"
                    </button>
                    <button
                        type=button_type
                        data-deve-mobile-touch-target="accessory_toolbar_buttons"
                        data-deve-mobile-toolbar-history-target=TOOLBAR_HISTORY_TARGET
                        data-deve-mobile-history-action=mobile_toolbar_history_actions_front_order()[1]
                        class=base
                        on:click=move |_| on_redo.run(())
                        disabled=disabled
                        title=move || t::common::redo(locale.get())
                        aria-label=move || t::common::redo(locale.get())
                    >
                        "↪"
                    </button>
                    <button
                        type=button_type
                        data-deve-mobile-touch-target="accessory_toolbar_buttons"
                        class=base
                        on:click=move |_| on_tab.run(())
                        disabled=disabled
                        title=move || t::common::tab(locale.get())
                        aria-label=move || t::common::tab(locale.get())
                    >
                        "⇥"
                    </button>
                    <button
                        type=button_type
                        data-deve-mobile-touch-target="accessory_toolbar_buttons"
                        class=base
                        on:click=move |_| on_h1.run(())
                        disabled=disabled
                        title=move || t::common::heading(locale.get())
                        aria-label=move || t::common::heading(locale.get())
                    >
                        "H"
                    </button>
                    <button
                        type=button_type
                        data-deve-mobile-touch-target="accessory_toolbar_buttons"
                        class=base
                        on:click=move |_| on_list.run(())
                        disabled=disabled
                        title=move || t::common::list(locale.get())
                        aria-label=move || t::common::list(locale.get())
                    >
                        "•"
                    </button>
                    <button
                        type=button_type
                        data-deve-mobile-touch-target="accessory_toolbar_buttons"
                        class=base
                        on:click=move |_| on_task.run(())
                        disabled=disabled
                        title=move || t::common::task(locale.get())
                        aria-label=move || t::common::task(locale.get())
                    >
                        "☑"
                    </button>
                    <button
                        type=button_type
                        data-deve-mobile-touch-target="accessory_toolbar_buttons"
                        class=base
                        on:click=move |_| on_bold.run(())
                        disabled=disabled
                        title=move || t::common::bold(locale.get())
                        aria-label=move || t::common::bold(locale.get())
                    >
                        "B"
                    </button>
                    <button
                        type=button_type
                        data-deve-mobile-touch-target="accessory_toolbar_buttons"
                        class=base
                        on:click=move |_| on_italic.run(())
                        disabled=disabled
                        title=move || t::common::italic(locale.get())
                        aria-label=move || t::common::italic(locale.get())
                    >
                        "I"
                    </button>
                    <button
                        type=button_type
                        data-deve-mobile-touch-target="accessory_toolbar_buttons"
                        class=base
                        on:click=move |_| on_code.run(())
                        disabled=disabled
                        title=move || t::common::code(locale.get())
                        aria-label=move || t::common::code(locale.get())
                    >
                        "<>"
                    </button>
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TOOLBAR_HISTORY_TARGET, mobile_toolbar_action_enabled,
        mobile_toolbar_history_actions_front_order, mobile_toolbar_style, toolbar_button_class,
        toolbar_button_type,
    };

    #[test]
    fn mobile_toolbar_keyboard_style_places_toolbar_above_keyboard() {
        assert_eq!(
            mobile_toolbar_style(312),
            "bottom: calc(312px + 0px + env(safe-area-inset-bottom));"
        );
    }

    #[test]
    fn mobile_touch_targets_accessory_toolbar_buttons_are_at_least_44px() {
        let class = toolbar_button_class();
        assert!(class.contains("h-11"));
        assert!(class.contains("min-w-[44px]"));
    }

    #[test]
    fn mobile_toolbar_write_gate_blocked_buttons_have_disabled_affordance() {
        let class = toolbar_button_class();
        assert!(class.contains("disabled:opacity-50"));
        assert!(class.contains("disabled:cursor-not-allowed"));
    }

    #[test]
    fn mobile_toolbar_history_actions_stay_front_loaded_for_390px() {
        assert_eq!(
            mobile_toolbar_history_actions_front_order(),
            ["undo", "redo"]
        );
        assert!(mobile_toolbar_history_actions_front_order().len() * 44 <= 390);
        assert_eq!(TOOLBAR_HISTORY_TARGET, "mobile_toolbar_history_actions");
    }

    #[test]
    fn mobile_toolbar_buttons_are_explicit_non_submit_buttons() {
        assert_eq!(toolbar_button_type(), "button");
    }

    #[test]
    fn mobile_toolbar_write_gate_blocks_actions() {
        assert!(mobile_toolbar_action_enabled(false));
        assert!(!mobile_toolbar_action_enabled(true));
    }
}

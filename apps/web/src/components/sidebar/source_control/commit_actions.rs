//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::components::icons::{Check, ChevronDown, Upload};
use crate::components::sidebar::source_control::touch_target::{
    commit_dropdown_button_class, commit_menu_item_class, commit_primary_button_class,
};
use crate::i18n::{Locale, t};
use leptos::ev::MouseEvent;
use leptos::prelude::*;

const COMMIT_DROPDOWN_BACKDROP_CLASS: &str = "fixed inset-0 z-[var(--z-floating)]";
const COMMIT_DROPDOWN_MENU_CLASS: &str = "absolute top-full left-0 right-0 mt-1 bg-dropdown border border-default rounded shadow-lg z-[calc(var(--z-floating)_+_1)] text-[13px]";

fn commit_dropdown_after_toggle_click(is_open: bool) -> bool {
    !is_open
}

fn commit_dropdown_after_outside_click() -> bool {
    false
}

#[component]
pub fn CommitActions(
    locale: RwSignal<Locale>,
    show_write_actions: Memo<bool>,
    can_prepare_commit: Memo<bool>,
    can_commit_now: Memo<bool>,
    prepare_commit_title: Memo<String>,
    commit_action_title: Memo<String>,
    dropdown_open: RwSignal<bool>,
    on_commit: Callback<()>,
    on_commit_and_push: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="flex relative">
            <button
                type="button"
                data-deve-source-control-commit-action="commit"
                class=move || commit_primary_button_class(show_write_actions.get())
                disabled=move || !can_commit_now.get()
                title=move || commit_action_title.get()
                on:click=move |_| {
                    dropdown_open.set(false);
                    on_commit.run(());
                }
            >
                <span class="codicon codicon-check"></span>
                <span>{move || t::source_control::commit(locale.get())}</span>
            </button>
            <Show when=move || show_write_actions.get()>
                <button
                    type="button"
                    class=commit_dropdown_button_class()
                    disabled=move || !can_prepare_commit.get()
                    aria-label=move || t::sidebar::more_actions(locale.get())
                    title=move || {
                        if can_prepare_commit.get() {
                            t::sidebar::more_actions(locale.get()).to_string()
                        } else {
                            prepare_commit_title.get()
                        }
                    }
                    aria-expanded=move || dropdown_open.get()
                    on:click=move |_| {
                        dropdown_open.update(|is_open| {
                            *is_open = commit_dropdown_after_toggle_click(*is_open);
                        });
                    }
                >
                    <ChevronDown class="w-3.5 h-3.5" />
                </button>
            </Show>
            {move || if show_write_actions.get() && dropdown_open.get() {
                view! {
                    <div
                        class=COMMIT_DROPDOWN_BACKDROP_CLASS
                        data-deve-source-control-commit-dropdown="outside"
                        on:click=move |ev: MouseEvent| {
                            ev.stop_propagation();
                            dropdown_open.set(commit_dropdown_after_outside_click());
                        }
                    ></div>
                    <div
                        class=COMMIT_DROPDOWN_MENU_CLASS
                        data-deve-source-control-commit-dropdown="menu"
                        on:click=move |ev: MouseEvent| ev.stop_propagation()
                    >
                        <button
                            type="button"
                            class=commit_menu_item_class()
                            disabled=move || !can_commit_now.get()
                            title=move || commit_action_title.get()
                            on:click=move |_| {
                                dropdown_open.set(false);
                                on_commit.run(());
                            }
                        >
                            <Check class="w-3.5 h-3.5" />
                            {move || t::source_control::commit(locale.get())}
                        </button>
                        <button
                            type="button"
                            class=commit_menu_item_class()
                            disabled=move || !can_commit_now.get()
                            title=move || commit_action_title.get()
                            on:click=move |_| {
                                dropdown_open.set(false);
                                on_commit_and_push.run(());
                            }
                        >
                            <Upload class="w-3.5 h-3.5" />
                            {move || t::source_control::commit_and_push(locale.get())}
                        </button>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        COMMIT_DROPDOWN_BACKDROP_CLASS, COMMIT_DROPDOWN_MENU_CLASS,
        commit_dropdown_after_outside_click, commit_dropdown_after_toggle_click,
    };

    #[test]
    fn commit_dropdown_toggle_click_inverts_state() {
        assert!(commit_dropdown_after_toggle_click(false));
        assert!(!commit_dropdown_after_toggle_click(true));
    }

    #[test]
    fn commit_dropdown_outside_click_closes_menu() {
        assert!(!commit_dropdown_after_outside_click());
    }

    #[test]
    fn commit_dropdown_menu_stays_above_backdrop() {
        assert!(COMMIT_DROPDOWN_BACKDROP_CLASS.contains("z-[var(--z-floating)]"));
        assert!(COMMIT_DROPDOWN_MENU_CLASS.contains("z-[calc(var(--z-floating)_+_1)]"));
    }
}

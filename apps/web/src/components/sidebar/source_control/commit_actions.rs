//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!
use crate::components::icons::{Check, ChevronDown, Upload};
use crate::components::sidebar::source_control::status_notice::blocked_title as blocked_status_title;
use crate::components::sidebar::source_control::touch_target::{
    commit_dropdown_button_class, commit_menu_item_class, commit_primary_button_class,
};
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(crate) fn commit_action_title(
    locale: Locale,
    write_block: Option<RepoWriteBlock>,
    can_prepare_commit: bool,
    can_commit_now: bool,
) -> String {
    if let Some(block) = write_block {
        return blocked_status_title(locale, block);
    }
    if !can_prepare_commit {
        return t::source_control::no_changes(locale).to_string();
    }
    if !can_commit_now {
        return t::source_control::commit_message_required(locale).to_string();
    }
    t::source_control::commit(locale).to_string()
}

#[component]
pub fn CommitActions(
    locale: RwSignal<Locale>,
    write_block: Signal<Option<RepoWriteBlock>>,
    show_write_actions: Signal<bool>,
    can_prepare_commit: Signal<bool>,
    can_commit_now: Signal<bool>,
    dropdown_open: RwSignal<bool>,
    on_commit: Callback<()>,
    on_commit_and_push: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="flex relative">
            <button
                class=move || commit_primary_button_class(show_write_actions.get())
                disabled=move || !can_commit_now.get()
                title=move || {
                    commit_action_title(
                        locale.get(),
                        write_block.get(),
                        can_prepare_commit.get(),
                        can_commit_now.get(),
                    )
                }
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
                    class=commit_dropdown_button_class()
                    disabled=move || !can_prepare_commit.get()
                    aria-label=move || t::sidebar::more_actions(locale.get())
                    title=move || {
                        if write_block.get().is_some() || !can_prepare_commit.get() {
                            commit_action_title(
                                locale.get(),
                                write_block.get(),
                                can_prepare_commit.get(),
                                can_commit_now.get(),
                            )
                        } else {
                            t::sidebar::more_actions(locale.get()).to_string()
                        }
                    }
                    on:click=move |_| dropdown_open.update(|is_open| *is_open = !*is_open)
                >
                    <ChevronDown class="w-3.5 h-3.5" />
                </button>
            </Show>
            {move || if show_write_actions.get() && dropdown_open.get() {
                view! {
                    <div class="absolute top-full left-0 right-0 mt-1 bg-dropdown border border-default rounded shadow-lg z-[var(--z-floating)] text-[13px]">
                        <button
                            class=commit_menu_item_class()
                            disabled=move || !can_commit_now.get()
                            title=move || {
                                commit_action_title(
                                    locale.get(),
                                    write_block.get(),
                                    can_prepare_commit.get(),
                                    can_commit_now.get(),
                                )
                            }
                            on:click=move |_| {
                                dropdown_open.set(false);
                                on_commit.run(());
                            }
                        >
                            <Check class="w-3.5 h-3.5" />
                            {move || t::source_control::commit(locale.get())}
                        </button>
                        <button
                            class=commit_menu_item_class()
                            disabled=move || !can_commit_now.get()
                            title=move || {
                                commit_action_title(
                                    locale.get(),
                                    write_block.get(),
                                    can_prepare_commit.get(),
                                    can_commit_now.get(),
                                )
                            }
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
    use super::commit_action_title;
    use crate::hooks::use_core::write_gate::RepoWriteBlock;
    use crate::i18n::{Locale, bottom_bar, source_control};

    #[test]
    fn commit_action_title_reports_empty_commit_source() {
        assert_eq!(
            commit_action_title(Locale::En, None, false, false),
            source_control::no_changes(Locale::En)
        );
    }

    #[test]
    fn commit_action_title_reports_missing_message() {
        assert_eq!(
            commit_action_title(Locale::Zh, None, true, false),
            source_control::commit_message_required(Locale::Zh)
        );
    }

    #[test]
    fn commit_action_title_prefers_write_block() {
        assert_eq!(
            commit_action_title(Locale::En, Some(RepoWriteBlock::ReadOnly), true, false),
            bottom_bar::read_only(Locale::En)
        );
    }

    #[test]
    fn commit_action_title_uses_action_label_when_enabled() {
        assert_eq!(
            commit_action_title(Locale::En, None, true, true),
            source_control::commit(Locale::En)
        );
    }
}

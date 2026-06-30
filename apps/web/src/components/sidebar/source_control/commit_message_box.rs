//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 16_ai_agent#native-ai-chat-runtime
//!
use crate::components::icons::Sparkles;
use crate::components::sidebar::source_control::status_notice::{
    blocked_hint as blocked_status_hint, blocked_title as blocked_status_title,
};
use crate::components::sidebar::source_control::touch_target::{
    commit_generate_button_class, commit_message_textarea_class,
};
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use web_sys::KeyboardEvent;

pub(crate) fn commit_message_placeholder_text(
    locale: Locale,
    write_block: Option<RepoWriteBlock>,
    has_commit_source: bool,
) -> &'static str {
    if let Some(block) = write_block {
        return blocked_status_hint(locale, block);
    }
    if !has_commit_source {
        return t::source_control::no_changes(locale);
    }
    t::source_control::commit_message_placeholder(locale)
}

#[component]
pub fn CommitMessageBox(
    locale: RwSignal<Locale>,
    write_block: Signal<Option<RepoWriteBlock>>,
    show_write_actions: Signal<bool>,
    has_commit_source: Signal<bool>,
    can_prepare_commit: Signal<bool>,
    msg: ReadSignal<String>,
    set_msg: WriteSignal<String>,
    is_generating: ReadSignal<bool>,
    on_keydown: Callback<KeyboardEvent>,
    on_generate: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="relative w-full">
            <textarea
                name="commit-message"
                class=commit_message_textarea_class()
                placeholder=move || {
                    commit_message_placeholder_text(
                        locale.get(),
                        write_block.get(),
                        has_commit_source.get(),
                    )
                }
                prop:value=msg
                on:input=move |ev| set_msg.set(event_target_value(&ev))
                on:keydown=move |ev| on_keydown.run(ev)
                disabled=move || !can_prepare_commit.get()
            />
            <Show when=move || show_write_actions.get()>
                <button
                    class=commit_generate_button_class()
                    aria-label=move || t::source_control::generate_commit_message(locale.get())
                    title=move || {
                        write_block
                            .get()
                            .map(|block| blocked_status_title(locale.get(), block))
                            .unwrap_or_else(|| t::source_control::generate_commit_message(locale.get()).to_string())
                    }
                    disabled=move || !can_prepare_commit.get() || is_generating.get()
                    on:click=move |_| on_generate.run(())
                >
                    <Sparkles class="w-3 h-3" />
                </button>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::commit_message_placeholder_text;
    use crate::hooks::use_core::write_gate::RepoWriteBlock;
    use crate::i18n::{Locale, source_control};

    #[test]
    fn commit_message_placeholder_reports_no_changes_when_commit_source_empty() {
        assert_eq!(
            commit_message_placeholder_text(Locale::En, None, false),
            source_control::no_changes(Locale::En)
        );
        assert_eq!(
            commit_message_placeholder_text(Locale::Zh, None, false),
            source_control::no_changes(Locale::Zh)
        );
    }

    #[test]
    fn commit_message_placeholder_prefers_write_block_hint_over_no_changes() {
        assert_eq!(
            commit_message_placeholder_text(Locale::En, Some(RepoWriteBlock::ReadOnly), false),
            source_control::readonly_write_gate_hint(Locale::En)
        );
    }

    #[test]
    fn commit_message_placeholder_uses_commit_message_when_commit_is_prepareable() {
        assert_eq!(
            commit_message_placeholder_text(Locale::En, None, true),
            source_control::commit_message_placeholder(Locale::En)
        );
    }
}

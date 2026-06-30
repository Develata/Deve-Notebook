//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 16_ai_agent#native-ai-chat-runtime
//!
use crate::components::icons::Sparkles;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[component]
pub fn CommitMessageBox(
    locale: RwSignal<Locale>,
    show_write_actions: Memo<bool>,
    can_prepare_commit: Memo<bool>,
    commit_input_placeholder: Memo<String>,
    prepare_commit_title: Memo<String>,
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
                class="w-full h-9 p-1.5 pr-9 text-[13px] bg-input border border-default rounded-[2px] focus:outline-none focus:border-b-accent focus:ring-1 focus:ring-accent placeholder:text-muted text-primary font-sans resize-none block leading-tight"
                placeholder=move || commit_input_placeholder.get()
                prop:value=msg
                on:input=move |ev| set_msg.set(event_target_value(&ev))
                on:keydown=move |ev| on_keydown.run(ev)
                disabled=move || !can_prepare_commit.get()
            />
            <Show when=move || show_write_actions.get()>
                <button
                    type="button"
                    class="absolute right-1 top-1 bottom-1 w-7 bg-accent hover:bg-accent-hover text-on-accent rounded flex items-center justify-center transition-colors z-[calc(var(--z-editor)_+_1)] disabled:opacity-50 disabled:cursor-not-allowed"
                    aria-label=move || t::source_control::generate_commit_message(locale.get())
                    title=move || {
                        prepare_commit_title.get()
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

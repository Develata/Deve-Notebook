//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 10_ai_agent#native-ai-chat-runtime
//!
use crate::components::icons::Sparkles;
use crate::components::sidebar::source_control::status_notice::{
    blocked_hint as blocked_status_hint, blocked_title as blocked_status_title,
};
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[component]
pub fn CommitMessageBox(
    locale: RwSignal<Locale>,
    write_block: Signal<Option<RepoWriteBlock>>,
    show_write_actions: Signal<bool>,
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
                class="w-full h-9 p-1.5 pr-20 text-[13px] bg-input border border-default rounded-[2px] focus:outline-none focus:border-b-accent focus:ring-1 focus:ring-accent placeholder:text-muted text-primary font-sans resize-none block leading-tight"
                placeholder=move || {
                    write_block
                        .get()
                        .map(|block| blocked_status_hint(locale.get(), block))
                        .unwrap_or_else(|| t::source_control::commit_message_placeholder(locale.get()))
                }
                prop:value=msg
                on:input=move |ev| set_msg.set(event_target_value(&ev))
                on:keydown=move |ev| on_keydown.run(ev)
                disabled=move || !can_prepare_commit.get()
            />
            <Show when=move || show_write_actions.get()>
                <button
                    class="absolute right-1 top-1 bottom-1 px-1.5 bg-accent hover:bg-accent-hover text-on-accent text-[10px] rounded flex items-center gap-1 transition-colors z-10 disabled:opacity-50 disabled:cursor-not-allowed"
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
                    {move || {
                        if is_generating.get() {
                            t::source_control::generating(locale.get())
                        } else {
                            t::source_control::generate(locale.get())
                        }
                    }}
                </button>
            </Show>
        </div>
    }
}

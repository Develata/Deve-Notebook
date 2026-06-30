//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 16_ai_agent#native-ai-chat-runtime
//!
use crate::components::icons::Sparkles;
use crate::components::sidebar::source_control::touch_target::{
    commit_generate_button_class, commit_message_textarea_class,
};
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
                class=commit_message_textarea_class()
                placeholder=move || commit_input_placeholder.get()
                prop:value=msg
                on:input=move |ev| set_msg.set(event_target_value(&ev))
                on:keydown=move |ev| on_keydown.run(ev)
                disabled=move || !can_prepare_commit.get()
            />
            <Show when=move || show_write_actions.get()>
                <button
                    type="button"
                    class=commit_generate_button_class()
                    aria-label=move || t::source_control::generate_commit_message(locale.get())
                    title=move || prepare_commit_title.get()
                    disabled=move || !can_prepare_commit.get() || is_generating.get()
                    on:click=move |_| on_generate.run(())
                >
                    <Sparkles class="w-3 h-3" />
                </button>
            </Show>
        </div>
    }
}

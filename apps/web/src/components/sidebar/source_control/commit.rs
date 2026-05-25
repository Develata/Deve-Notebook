//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::components::sidebar::source_control::commit_actions::CommitActions;
use crate::components::sidebar::source_control::commit_controller::use_commit_controller;
use crate::components::sidebar::source_control::commit_message_box::CommitMessageBox;
use crate::hooks::use_core::{ChatContext, SourceControlContext};
use crate::i18n::Locale;
use leptos::prelude::*;

#[component]
pub fn Commit() -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let chat_ctx = expect_context::<ChatContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let controller = use_commit_controller(core, chat_ctx, locale);

    view! {
        <div class="border-t border-default px-2 py-2">
            <div class="flex flex-col gap-2">
                <CommitMessageBox
                    locale
                    write_block=controller.write_block
                    show_write_actions=controller.show_write_actions
                    can_prepare_commit=controller.can_prepare_commit
                    msg=controller.msg
                    set_msg=controller.set_msg
                    is_generating=controller.is_generating
                    on_keydown=controller.on_keydown
                    on_generate=controller.on_generate
                />

                <CommitActions
                    locale
                    write_block=controller.write_block
                    show_write_actions=controller.show_write_actions
                    can_prepare_commit=controller.can_prepare_commit
                    can_commit_now=controller.can_commit_now
                    dropdown_open=controller.dropdown_open
                    on_commit=controller.on_commit
                    on_commit_and_push=controller.on_commit_and_push
                />
            </div>
        </div>
    }
}

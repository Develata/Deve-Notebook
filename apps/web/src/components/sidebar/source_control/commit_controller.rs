use crate::components::sidebar::source_control::commit_ai::{
    build_generate_callback, sync_generated_commit_message,
};
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::hooks::use_core::{ChatContext, SourceControlContext};
use crate::i18n::Locale;
use leptos::prelude::*;
use web_sys::KeyboardEvent;

pub struct CommitController {
    pub msg: ReadSignal<String>,
    pub set_msg: WriteSignal<String>,
    pub is_generating: ReadSignal<bool>,
    pub dropdown_open: RwSignal<bool>,
    pub write_block: Signal<Option<RepoWriteBlock>>,
    pub show_write_actions: Signal<bool>,
    pub can_prepare_commit: Signal<bool>,
    pub can_commit_now: Signal<bool>,
    pub on_keydown: Callback<KeyboardEvent>,
    pub on_generate: Callback<()>,
    pub on_commit: Callback<()>,
    pub on_commit_and_push: Callback<()>,
}

pub fn use_commit_controller(
    core: SourceControlContext,
    chat_ctx: ChatContext,
    locale: RwSignal<Locale>,
) -> CommitController {
    let (msg, set_msg) = signal(String::new());
    let (is_generating, set_is_generating) = signal(false);
    let dropdown_open = RwSignal::new(false);
    let active_req_id = RwSignal::new(None::<String>);
    let saw_streaming = RwSignal::new(false);
    let write_block = core.write_block;
    let show_write_actions = Signal::derive(move || write_block.get().is_none());
    let has_staged = Signal::derive(move || !core.staged_changes.get().is_empty());
    let can_prepare_commit =
        Signal::derive(move || core.can_write.get() && has_staged.get());
    let can_commit_now =
        Signal::derive(move || can_prepare_commit.get() && !msg.get().trim().is_empty());

    let on_keydown = Callback::new({
        let core = core.clone();
        move |ev: KeyboardEvent| {
            if ev.ctrl_key()
                && ev.key() == "Enter"
                && core.can_write.get_untracked()
                && !core.staged_changes.get_untracked().is_empty()
                && !msg.get_untracked().trim().is_empty()
            {
                dropdown_open.set(false);
                core.clear_notice.run(());
                core.on_commit.run(msg.get_untracked());
                set_msg.set(String::new());
            }
        }
    });

    let on_generate = build_generate_callback(
        core.clone(),
        chat_ctx.clone(),
        locale,
        active_req_id,
        saw_streaming,
        set_is_generating,
    );

    let on_commit = Callback::new({
        let core = core.clone();
        move |_| {
            if !(core.can_write.get_untracked()
                && !core.staged_changes.get_untracked().is_empty()
                && !msg.get_untracked().trim().is_empty())
            {
                return;
            }
            dropdown_open.set(false);
            core.clear_notice.run(());
            core.on_commit.run(msg.get_untracked());
            set_msg.set(String::new());
        }
    });

    let on_commit_and_push = Callback::new({
        let core = core.clone();
        move |_| {
            if !(core.can_write.get_untracked()
                && !core.staged_changes.get_untracked().is_empty()
                && !msg.get_untracked().trim().is_empty())
            {
                return;
            }
            dropdown_open.set(false);
            core.clear_notice.run(());
            core.on_commit_and_push.run(msg.get_untracked());
            set_msg.set(String::new());
        }
    });

    sync_generated_commit_message(
        chat_ctx,
        active_req_id,
        saw_streaming,
        set_msg,
        set_is_generating,
    );

    CommitController {
        msg,
        set_msg,
        is_generating,
        dropdown_open,
        write_block,
        show_write_actions,
        can_prepare_commit,
        can_commit_now,
        on_keydown,
        on_generate,
        on_commit,
        on_commit_and_push,
    }
}

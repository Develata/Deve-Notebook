//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::components::sidebar::source_control::commit_ai::{
    build_generate_callback, sync_generated_commit_message,
};
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
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
    pub show_write_actions: Memo<bool>,
    pub can_prepare_commit: Memo<bool>,
    pub can_commit_now: Memo<bool>,
    pub on_keydown: Callback<KeyboardEvent>,
    pub on_generate: Callback<()>,
    pub on_commit: Callback<()>,
    pub on_commit_and_push: Callback<()>,
}

fn can_submit_commit_now(core: &SourceControlContext, message: &str) -> bool {
    core.can_write.get_untracked()
        && (!core.staged_changes.get_untracked().is_empty()
            || !core.confirmed_changes.get_untracked().is_empty())
        && !message.trim().is_empty()
}

fn show_git_push_cli_only_notice(set_notice: WriteSignal<Option<SourceControlNotice>>) {
    set_notice.set(Some(SourceControlNotice::git_push_cli_only()));
}

fn commit_submit_shortcut_pressed(ctrl_key: bool, meta_key: bool, key: &str) -> bool {
    (ctrl_key || meta_key) && key == "Enter"
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
    let show_write_actions = Memo::new(move |_| write_block.get().is_none());
    let can_prepare_commit = Memo::new(move |_| {
        core.can_write.get()
            && (!core.staged_changes.get().is_empty() || !core.confirmed_changes.get().is_empty())
    });
    let can_commit_now =
        Memo::new(move |_| can_prepare_commit.get() && !msg.get().trim().is_empty());

    let on_keydown = Callback::new({
        let core = core.clone();
        move |ev: KeyboardEvent| {
            if commit_submit_shortcut_pressed(ev.ctrl_key(), ev.meta_key(), &ev.key()) {
                ev.prevent_default();
                if !can_submit_commit_now(&core, &msg.get_untracked()) {
                    return;
                }
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
            if !can_submit_commit_now(&core, &msg.get_untracked()) {
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
            if !can_submit_commit_now(&core, &msg.get_untracked()) {
                return;
            }
            dropdown_open.set(false);
            show_git_push_cli_only_notice(core.set_notice);
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

#[cfg(test)]
mod tests {
    use super::{commit_submit_shortcut_pressed, show_git_push_cli_only_notice};
    use crate::hooks::use_core::source_control_notice::is_git_push_cli_notice;
    use leptos::prelude::{GetUntracked, signal};

    #[test]
    fn commit_shortcut_supports_ctrl_and_meta_enter() {
        assert!(commit_submit_shortcut_pressed(true, false, "Enter"));
        assert!(commit_submit_shortcut_pressed(false, true, "Enter"));
        assert!(commit_submit_shortcut_pressed(true, true, "Enter"));
        assert!(!commit_submit_shortcut_pressed(false, false, "Enter"));
        assert!(!commit_submit_shortcut_pressed(true, false, "N"));
    }

    #[test]
    fn commit_and_push_sets_git_push_cli_only_notice() {
        let (notice, set_notice) = signal(None);

        show_git_push_cli_only_notice(set_notice);

        assert!(
            notice
                .get_untracked()
                .as_ref()
                .is_some_and(is_git_push_cli_notice)
        );
    }
}

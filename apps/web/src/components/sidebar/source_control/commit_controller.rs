//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::components::sidebar::source_control::commit_ai::{
    build_generate_callback, sync_generated_commit_message,
};
use crate::components::sidebar::source_control::status_notice::{
    blocked_hint as blocked_status_hint, blocked_title as blocked_status_title,
};
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::hooks::use_core::{ChatContext, SourceControlContext};
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use web_sys::KeyboardEvent;

pub struct CommitController {
    pub msg: ReadSignal<String>,
    pub set_msg: WriteSignal<String>,
    pub is_generating: ReadSignal<bool>,
    pub dropdown_open: RwSignal<bool>,
    pub show_write_actions: Memo<bool>,
    pub can_prepare_commit: Memo<bool>,
    pub can_commit_now: Memo<bool>,
    pub commit_input_placeholder: Memo<String>,
    pub prepare_commit_title: Memo<String>,
    pub commit_action_title: Memo<String>,
    pub on_keydown: Callback<KeyboardEvent>,
    pub on_generate: Callback<()>,
    pub on_commit: Callback<()>,
    pub on_commit_and_push: Callback<()>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommitDisabledReason {
    WriteBlock(RepoWriteBlock),
    NoChanges,
    EmptyMessage,
}

fn can_submit_commit_now(core: &SourceControlContext, message: &str) -> bool {
    core.can_write.get_untracked()
        && !core.confirmed_changes.get_untracked().is_empty()
        && !message.trim().is_empty()
}

fn commit_disabled_reason(
    write_block: Option<RepoWriteBlock>,
    has_changes: bool,
    message: &str,
) -> Option<CommitDisabledReason> {
    if let Some(block) = write_block {
        return Some(CommitDisabledReason::WriteBlock(block));
    }
    if !has_changes {
        return Some(CommitDisabledReason::NoChanges);
    }
    if message.trim().is_empty() {
        return Some(CommitDisabledReason::EmptyMessage);
    }
    None
}

fn commit_disabled_title(locale: Locale, reason: CommitDisabledReason) -> String {
    match reason {
        CommitDisabledReason::WriteBlock(block) => blocked_status_title(locale, block),
        CommitDisabledReason::NoChanges => {
            t::source_control::commit_disabled_no_changes(locale).to_string()
        }
        CommitDisabledReason::EmptyMessage => {
            t::source_control::commit_disabled_empty_message(locale).to_string()
        }
    }
}

fn commit_input_placeholder(locale: Locale, reason: Option<CommitDisabledReason>) -> String {
    match reason {
        Some(CommitDisabledReason::WriteBlock(block)) => {
            blocked_status_hint(locale, block).to_string()
        }
        Some(CommitDisabledReason::NoChanges) => {
            t::source_control::commit_disabled_no_changes_hint(locale).to_string()
        }
        Some(CommitDisabledReason::EmptyMessage) | None => {
            t::source_control::commit_message_placeholder(locale).to_string()
        }
    }
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
    let has_commit_changes = Memo::new(move |_| !core.confirmed_changes.get().is_empty());
    let can_prepare_commit = Memo::new(move |_| core.can_write.get() && has_commit_changes.get());
    let can_commit_now =
        Memo::new(move |_| can_prepare_commit.get() && !msg.get().trim().is_empty());
    let commit_disabled_reason = Memo::new(move |_| {
        commit_disabled_reason(write_block.get(), has_commit_changes.get(), &msg.get())
    });
    let commit_input_placeholder =
        Memo::new(move |_| commit_input_placeholder(locale.get(), commit_disabled_reason.get()));
    let prepare_commit_title = Memo::new(move |_| {
        commit_disabled_reason
            .get()
            .map(|reason| commit_disabled_title(locale.get(), reason))
            .unwrap_or_else(|| t::source_control::generate_commit_message(locale.get()).to_string())
    });
    let commit_action_title = Memo::new(move |_| {
        commit_disabled_reason
            .get()
            .map(|reason| commit_disabled_title(locale.get(), reason))
            .unwrap_or_else(|| t::source_control::commit(locale.get()).to_string())
    });

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
        show_write_actions,
        can_prepare_commit,
        can_commit_now,
        commit_input_placeholder,
        prepare_commit_title,
        commit_action_title,
        on_keydown,
        on_generate,
        on_commit,
        on_commit_and_push,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommitDisabledReason, commit_disabled_reason, commit_disabled_title,
        commit_input_placeholder, commit_submit_shortcut_pressed, show_git_push_cli_only_notice,
    };
    use crate::hooks::use_core::source_control_notice::is_git_push_cli_notice;
    use crate::hooks::use_core::write_gate::RepoWriteBlock;
    use crate::i18n::{Locale, t};
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

    #[test]
    fn commit_disabled_reason_prioritizes_write_gate_then_changes_then_message() {
        assert_eq!(
            commit_disabled_reason(Some(RepoWriteBlock::ReadOnly), false, ""),
            Some(CommitDisabledReason::WriteBlock(RepoWriteBlock::ReadOnly))
        );
        assert_eq!(
            commit_disabled_reason(None, false, ""),
            Some(CommitDisabledReason::NoChanges)
        );
        assert_eq!(
            commit_disabled_reason(None, true, "  "),
            Some(CommitDisabledReason::EmptyMessage)
        );
        assert_eq!(commit_disabled_reason(None, true, "ship it"), None);
    }

    #[test]
    fn commit_disabled_copy_is_structured_for_local_commit_states() {
        assert_eq!(
            commit_disabled_title(Locale::Zh, CommitDisabledReason::NoChanges),
            t::source_control::commit_disabled_no_changes(Locale::Zh)
        );
        assert_eq!(
            commit_disabled_title(Locale::En, CommitDisabledReason::EmptyMessage),
            "Enter a commit message before committing"
        );
        assert_eq!(
            commit_input_placeholder(Locale::Zh, Some(CommitDisabledReason::NoChanges)),
            t::source_control::commit_disabled_no_changes_hint(Locale::Zh)
        );
    }
}

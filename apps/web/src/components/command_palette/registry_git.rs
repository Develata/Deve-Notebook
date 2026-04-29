//! plan_ref:
//!   - 12_commands#command-palette-shortcuts
//!   - 07_diff_logic#git-mirror-lifecycle
//!
use crate::components::command_palette::types::Command;
use crate::hooks::use_core::{SourceControlContext, source_control_notice::SourceControlNotice};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

fn show_source_control_notice(notice: SourceControlNotice, set_show: WriteSignal<bool>) {
    if let Some(source_control) = use_context::<SourceControlContext>() {
        source_control.set_notice.set(Some(notice));
    }
    set_show.set(false);
}

pub(super) fn git_import_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    Command {
        id: "git_import_changes".to_string(),
        title: (t::command_palette::git_import_changes)(locale).to_string(),
        action: Callback::new(move |_| {
            show_source_control_notice(SourceControlNotice::git_import_cli_only(), set_show);
        }),
        is_file: false,
    }
}

pub(super) fn git_push_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    Command {
        id: "git_push_mirror".to_string(),
        title: (t::command_palette::git_push_mirror)(locale).to_string(),
        action: Callback::new(move |_| {
            show_source_control_notice(SourceControlNotice::git_push_cli_only(), set_show);
        }),
        is_file: false,
    }
}

pub(super) fn git_repair_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    Command {
        id: "git_repair_mirror".to_string(),
        title: (t::command_palette::git_repair_mirror)(locale).to_string(),
        action: Callback::new(move |_| {
            show_source_control_notice(SourceControlNotice::git_repair_cli_only(), set_show);
        }),
        is_file: false,
    }
}

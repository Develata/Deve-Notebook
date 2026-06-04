//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 05_diff_logic#git-mirror-lifecycle
//!
use crate::components::command_palette::types::Command;
use crate::hooks::use_core::{SourceControlContext, source_control_notice::SourceControlNotice};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

fn show_source_control_notice(
    source_control: Option<SourceControlContext>,
    notice: SourceControlNotice,
    set_show: WriteSignal<bool>,
) {
    if let Some(source_control) = source_control {
        source_control.set_notice.set(Some(notice));
    }
    set_show.set(false);
}

pub(super) fn git_import_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    let source_control = use_context::<SourceControlContext>();
    Command::available(
        "git_import_changes",
        (t::command_palette::git_import_changes)(locale),
        Callback::new(move |_| {
            show_source_control_notice(
                source_control.clone(),
                SourceControlNotice::git_import_cli_only(),
                set_show,
            );
        }),
    )
    .with_group((t::command_palette::group_git)(locale))
    .with_enabled_when((t::command_palette::enabled_cli_only_notice)(locale))
}

pub(super) fn git_status_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    let source_control = use_context::<SourceControlContext>();
    Command::unavailable(
        "git_status",
        (t::command_palette::git_status)(locale),
        (t::command_palette::git_cli_only_reason)(locale),
        Callback::new(move |_| {
            show_source_control_notice(
                source_control.clone(),
                SourceControlNotice::git_status_cli_only(),
                set_show,
            );
        }),
    )
    .with_group((t::command_palette::group_git)(locale))
    .with_enabled_when((t::command_palette::enabled_cli_only_notice)(locale))
}

pub(super) fn git_mirror_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    let source_control = use_context::<SourceControlContext>();
    Command::unavailable(
        "git_mirror",
        (t::command_palette::git_mirror)(locale),
        (t::command_palette::git_cli_only_reason)(locale),
        Callback::new(move |_| {
            show_source_control_notice(
                source_control.clone(),
                SourceControlNotice::git_mirror_cli_only(),
                set_show,
            );
        }),
    )
    .with_group((t::command_palette::group_git)(locale))
    .with_enabled_when((t::command_palette::enabled_cli_only_notice)(locale))
}

pub(super) fn git_export_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    let source_control = use_context::<SourceControlContext>();
    Command::unavailable(
        "git_export_mirror",
        (t::command_palette::git_export_mirror)(locale),
        (t::command_palette::git_cli_only_reason)(locale),
        Callback::new(move |_| {
            show_source_control_notice(
                source_control.clone(),
                SourceControlNotice::git_export_cli_only(),
                set_show,
            );
        }),
    )
    .with_group((t::command_palette::group_git)(locale))
    .with_enabled_when((t::command_palette::enabled_cli_only_notice)(locale))
}

pub(super) fn git_push_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    let source_control = use_context::<SourceControlContext>();
    Command::available(
        "git_push_mirror",
        (t::command_palette::git_push_mirror)(locale),
        Callback::new(move |_| {
            show_source_control_notice(
                source_control.clone(),
                SourceControlNotice::git_push_cli_only(),
                set_show,
            );
        }),
    )
    .with_group((t::command_palette::group_git)(locale))
    .with_enabled_when((t::command_palette::enabled_cli_only_notice)(locale))
}

pub(super) fn git_repair_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    let source_control = use_context::<SourceControlContext>();
    Command::available(
        "git_repair_mirror",
        (t::command_palette::git_repair_mirror)(locale),
        Callback::new(move |_| {
            show_source_control_notice(
                source_control.clone(),
                SourceControlNotice::git_repair_cli_only(),
                set_show,
            );
        }),
    )
    .with_group((t::command_palette::group_git)(locale))
    .with_enabled_when((t::command_palette::enabled_cli_only_notice)(locale))
}

#[cfg(test)]
mod tests;

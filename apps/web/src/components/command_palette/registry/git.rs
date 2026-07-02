//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 05_diff_logic#git-mirror-lifecycle
//!
use crate::components::command_palette::types::Command;
use crate::hooks::use_core::{SourceControlContext, source_control_notice::SourceControlNotice};
use crate::i18n::{Locale, t};
use crate::runtime::session_client::SessionClient;
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

fn git_bridge_enabled_when(locale: Locale) -> String {
    let mode = use_context::<SessionClient>()
        .map(|session| git_bridge_mode_from_value(&session.ws.source_control_git_bridge.get()))
        .unwrap_or("unknown");
    format!(
        "{}; source_control.git_bridge={mode}",
        (t::command_palette::enabled_cli_only_notice)(locale)
    )
}

fn git_bridge_mode_from_value(mode: &str) -> &'static str {
    match mode {
        "mirror" => "mirror",
        "off" => "off",
        "unknown" => "unknown",
        _ => "unknown",
    }
}

pub(super) fn git_import_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    source_control: Option<SourceControlContext>,
) -> Command {
    Command::unavailable(
        "git_import_changes",
        (t::command_palette::git_import_changes)(locale),
        (t::command_palette::git_cli_only_reason)(locale),
        Callback::new(move |_| {
            show_source_control_notice(
                source_control.clone(),
                SourceControlNotice::git_import_cli_only(),
                set_show,
            );
        }),
    )
    .with_group((t::command_palette::group_git)(locale))
    .with_enabled_when(git_bridge_enabled_when(locale))
}

pub(super) fn git_status_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    source_control: Option<SourceControlContext>,
) -> Command {
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
    .with_enabled_when(git_bridge_enabled_when(locale))
}

pub(super) fn git_mirror_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    source_control: Option<SourceControlContext>,
) -> Command {
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
    .with_enabled_when(git_bridge_enabled_when(locale))
}

pub(super) fn git_export_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    source_control: Option<SourceControlContext>,
) -> Command {
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
    .with_enabled_when(git_bridge_enabled_when(locale))
}

pub(super) fn git_push_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    source_control: Option<SourceControlContext>,
) -> Command {
    Command::unavailable(
        "git_push_mirror",
        (t::command_palette::git_push_mirror)(locale),
        (t::command_palette::git_cli_only_reason)(locale),
        Callback::new(move |_| {
            show_source_control_notice(
                source_control.clone(),
                SourceControlNotice::git_push_cli_only(),
                set_show,
            );
        }),
    )
    .with_group((t::command_palette::group_git)(locale))
    .with_enabled_when(git_bridge_enabled_when(locale))
}

pub(super) fn git_repair_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    source_control: Option<SourceControlContext>,
) -> Command {
    Command::unavailable(
        "git_repair_mirror",
        (t::command_palette::git_repair_mirror)(locale),
        (t::command_palette::git_cli_only_reason)(locale),
        Callback::new(move |_| {
            show_source_control_notice(
                source_control.clone(),
                SourceControlNotice::git_repair_cli_only(),
                set_show,
            );
        }),
    )
    .with_group((t::command_palette::group_git)(locale))
    .with_enabled_when(git_bridge_enabled_when(locale))
}

#[cfg(test)]
mod tests;

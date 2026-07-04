//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 05_diff_logic#git-mirror-lifecycle
//!
mod notice;

use self::notice::{show_ngit_status_notice, show_source_control_notice};
use crate::components::command_palette::types::Command;
use crate::components::main_layout::SidebarControl;
use crate::hooks::use_core::{SourceControlContext, source_control_notice::SourceControlNotice};
use crate::i18n::{Locale, t};
use crate::runtime::session_client::SessionClient;
use leptos::prelude::*;

#[cfg(test)]
use self::notice::show_ngit_status_notice_for_viewport;

fn ngit_enabled_when(locale: Locale) -> String {
    let authority = use_context::<SessionClient>()
        .map(|session| {
            source_control_authority_from_value(&session.ws.source_control_authority.get())
        })
        .unwrap_or("unknown");
    format!(
        "{}; source_control.authority={authority}",
        (t::command_palette::enabled_cli_only_notice)(locale)
    )
}

fn source_control_authority_from_value(authority: &str) -> &'static str {
    match authority {
        "ngit" => "ngit",
        "unknown" => "unknown",
        _ => "unknown",
    }
}

pub(super) fn ngit_import_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
) -> Command {
    Command::unavailable(
        "ngit_import_changes",
        (t::command_palette::ngit_import_changes)(locale),
        (t::command_palette::ngit_cli_only_reason)(locale),
        move || {
            show_source_control_notice(
                set_notice,
                sidebar_control,
                SourceControlNotice::git_import_cli_only(),
                set_show,
            );
        },
    )
    .with_group((t::command_palette::group_ngit)(locale))
    .with_enabled_when(ngit_enabled_when(locale))
}

pub(super) fn ngit_status_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
) -> Command {
    let source_control = use_context::<SourceControlContext>();
    Command::unavailable(
        "ngit_status",
        (t::command_palette::ngit_status)(locale),
        (t::command_palette::ngit_cli_only_reason)(locale),
        move || {
            show_ngit_status_notice(
                set_notice,
                sidebar_control,
                source_control.clone(),
                set_show,
            );
        },
    )
    .with_group((t::command_palette::group_ngit)(locale))
    .with_enabled_when(ngit_enabled_when(locale))
}

pub(super) fn ngit_mirror_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
) -> Command {
    Command::unavailable(
        "ngit_mirror",
        (t::command_palette::ngit_mirror)(locale),
        (t::command_palette::ngit_cli_only_reason)(locale),
        move || {
            show_source_control_notice(
                set_notice,
                sidebar_control,
                SourceControlNotice::git_mirror_cli_only(),
                set_show,
            );
        },
    )
    .with_group((t::command_palette::group_ngit)(locale))
    .with_enabled_when(ngit_enabled_when(locale))
}

pub(super) fn ngit_export_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
) -> Command {
    Command::unavailable(
        "ngit_export_mirror",
        (t::command_palette::ngit_export_mirror)(locale),
        (t::command_palette::ngit_cli_only_reason)(locale),
        move || {
            show_source_control_notice(
                set_notice,
                sidebar_control,
                SourceControlNotice::git_export_cli_only(),
                set_show,
            );
        },
    )
    .with_group((t::command_palette::group_ngit)(locale))
    .with_enabled_when(ngit_enabled_when(locale))
}

pub(super) fn ngit_push_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
) -> Command {
    Command::unavailable(
        "ngit_push_mirror",
        (t::command_palette::ngit_push_mirror)(locale),
        (t::command_palette::ngit_cli_only_reason)(locale),
        move || {
            show_source_control_notice(
                set_notice,
                sidebar_control,
                SourceControlNotice::git_push_cli_only(),
                set_show,
            );
        },
    )
    .with_group((t::command_palette::group_ngit)(locale))
    .with_enabled_when(ngit_enabled_when(locale))
}

pub(super) fn ngit_repair_command(
    locale: Locale,
    set_show: WriteSignal<bool>,
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
) -> Command {
    Command::unavailable(
        "ngit_repair_mirror",
        (t::command_palette::ngit_repair_mirror)(locale),
        (t::command_palette::ngit_cli_only_reason)(locale),
        move || {
            show_source_control_notice(
                set_notice,
                sidebar_control,
                SourceControlNotice::git_repair_cli_only(),
                set_show,
            );
        },
    )
    .with_group((t::command_palette::group_ngit)(locale))
    .with_enabled_when(ngit_enabled_when(locale))
}

#[cfg(test)]
mod tests;

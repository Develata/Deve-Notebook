use super::super::{
    ngit_export_command, ngit_import_command, ngit_mirror_command, ngit_push_command,
    ngit_repair_command,
};
use super::support::{
    assert_cli_notice_command, command_contexts, provide_sidebar_control_context,
    provide_source_control_context,
};
use crate::components::activity_bar::SidebarView;
use crate::hooks::use_core::source_control_notice::{
    is_git_export_cli_notice, is_git_import_cli_notice, is_git_mirror_cli_notice,
    is_git_push_cli_notice, is_git_repair_cli_notice,
};
use crate::i18n::Locale;
use leptos::prelude::{GetUntracked, signal};
use leptos::reactive::owner::Owner;

#[test]
fn ngit_import_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_import_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_import_cli_notice);
    });
}

#[test]
fn ngit_mirror_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_mirror_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_mirror_cli_notice);
    });
}

#[test]
fn ngit_export_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_export_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_export_cli_notice);
    });
}

#[test]
fn ngit_push_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_push_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_push_cli_notice);
    });
}

#[test]
fn ngit_push_command_routes_notice_to_source_control_sidebar() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (sidebar_visible, mobile_visible, active_view) = provide_sidebar_control_context(false);
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_push_command(Locale::En, set_show, source_control, sidebar_control);

        assert_cli_notice_command(command, show, notice, is_git_push_cli_notice);
        assert!(sidebar_visible.get_untracked());
        assert!(!mobile_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
    });
}

#[test]
fn ngit_push_command_routes_notice_to_mobile_source_control_drawer() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (sidebar_visible, mobile_visible, active_view) = provide_sidebar_control_context(true);
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_push_command(Locale::En, set_show, source_control, sidebar_control);

        assert_cli_notice_command(command, show, notice, is_git_push_cli_notice);
        assert!(!sidebar_visible.get_untracked());
        assert!(mobile_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
    });
}

#[test]
fn ngit_repair_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_repair_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_repair_cli_notice);
    });
}

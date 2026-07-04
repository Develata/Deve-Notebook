use super::super::{ngit_status_command, show_ngit_status_notice_for_viewport};
use super::support::{
    assert_cli_notice_command, command_contexts, create_commands, ngit_status_enabled_when,
    provide_session_client, provide_sidebar_control_context, provide_source_control_context,
};
use crate::api::{ConnectionStatus, WsService};
use crate::components::activity_bar::SidebarView;
use crate::hooks::use_core::SourceControlContext;
use crate::hooks::use_core::source_control_notice::{
    SourceControlNotice, is_git_status_cli_notice,
};
use crate::i18n::Locale;
use leptos::prelude::{GetUntracked, Set, signal, use_context};
use leptos::reactive::owner::Owner;

#[test]
fn ngit_status_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_status_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert!(
            command
                .enabled_when
                .contains("source_control.authority=unknown")
        );
        assert_cli_notice_command(command, show, notice, is_git_status_cli_notice);
    });
}

#[test]
fn ngit_status_command_routes_mobile_to_source_control_drawer() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (sidebar_visible, mobile_visible, active_view) = provide_sidebar_control_context(true);
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_status_command(Locale::En, set_show, source_control, sidebar_control);

        command.action.run(());

        assert!(!show.get_untracked());
        assert!(notice.get_untracked().is_none());
        assert!(!sidebar_visible.get_untracked());
        assert!(mobile_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
    });
}

#[test]
fn ngit_status_command_clears_stale_cli_notice_on_mobile() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let source_control = use_context::<SourceControlContext>().expect("source control context");
        source_control
            .set_notice
            .set(Some(SourceControlNotice::git_status_cli_only()));
        let (_, mobile_visible, active_view) = provide_sidebar_control_context(true);
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_status_command(Locale::En, set_show, source_control, sidebar_control);

        command.action.run(());

        assert!(!show.get_untracked());
        assert!(notice.get_untracked().is_none());
        assert!(mobile_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
    });
}

#[test]
fn ngit_status_command_does_not_write_notice_on_mobile_viewport_without_sidebar_context() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();

        assert!(sidebar_control.is_none());
        show_ngit_status_notice_for_viewport(
            source_control,
            sidebar_control,
            use_context::<SourceControlContext>(),
            set_show,
            true,
        );

        assert!(!show.get_untracked());
        assert!(notice.get_untracked().is_none());
    });
}

#[test]
fn ngit_status_command_keeps_desktop_cli_notice_without_sidebar_context() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();

        assert!(sidebar_control.is_none());
        show_ngit_status_notice_for_viewport(
            source_control,
            sidebar_control,
            use_context::<SourceControlContext>(),
            set_show,
            false,
        );

        assert!(!show.get_untracked());
        let notice = notice.get_untracked().expect("source control notice");
        assert!(is_git_status_cli_notice(&notice));
    });
}

#[test]
fn ngit_status_command_routes_notice_to_source_control_sidebar_on_desktop() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (sidebar_visible, mobile_visible, active_view) = provide_sidebar_control_context(false);
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_status_command(Locale::En, set_show, source_control, sidebar_control);

        assert_cli_notice_command(command, show, notice, is_git_status_cli_notice);
        assert!(sidebar_visible.get_untracked());
        assert!(!mobile_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
    });
}

#[test]
fn ngit_status_detail_text_exposes_ngit_authority() {
    let owner = Owner::new();
    owner.with(|| {
        provide_source_control_context();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        ws.complete_foreground_node_role_reprobe("main", "ngit", false, false);
        provide_session_client(ws);
        let (show, set_show) = signal(true);
        let command = create_commands(set_show)
            .get_untracked()
            .into_iter()
            .find(|command| command.id == "ngit_status")
            .expect("ngit status command");

        let reason = command.availability.reason().expect("unavailable reason");
        let detail = command.detail_text();

        assert!(detail.contains(reason));
        assert!(detail.contains("source_control.authority=ngit"));
        assert!(show.get_untracked());
    });
}

#[test]
fn command_palette_source_control_authority_reads_session_signal() {
    let owner = Owner::new();
    owner.with(|| {
        provide_source_control_context();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        ws.complete_foreground_node_role_reprobe("main", "ngit", false, false);
        provide_session_client(ws);
        let (_, set_show) = signal(true);

        let enabled_when = ngit_status_enabled_when(create_commands(set_show));

        assert!(enabled_when.contains("source_control.authority=ngit"));
    });
}

#[test]
fn command_palette_source_control_authority_updates_after_node_role_probe() {
    let owner = Owner::new();
    owner.with(|| {
        provide_source_control_context();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        provide_session_client(ws.clone());
        let (_, set_show) = signal(true);
        let commands = create_commands(set_show);

        assert!(ngit_status_enabled_when(commands).contains("source_control.authority=unknown"));

        ws.complete_foreground_node_role_reprobe("main", "ngit", false, false);

        assert!(ngit_status_enabled_when(commands).contains("source_control.authority=ngit"));
    });
}

//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 05_diff_logic#remote-projection-transport

use super::remote_projection_commands;
use crate::api::{ConnectionStatus, WsService};
use crate::components::activity_bar::SidebarView;
use crate::components::layout_context::SidebarControl;
use crate::i18n::Locale;
use crate::runtime::session_client::SessionClient;
use deve_core::protocol::{ClientMessage, RemoteProjectionDirection, RemoteProjectionProvider};
use leptos::prelude::*;

#[test]
fn remote_projection_commands_are_backend_intent_entries() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let (_, set_show) = signal(true);
        let commands = remote_projection_commands(Locale::En, set_show, None, None);
        let ids = commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, ["webdav:push", "webdav:pull", "s3:push", "s3:pull"]);
        assert!(
            commands
                .iter()
                .all(|command| !command.availability.is_unavailable())
        );
        assert!(commands[1].title.contains("webdav:pull"));
        assert!(commands[1].detail_text().contains("current repo URL"));
        assert!(commands[1].detail_text().contains("External Changes"));
    });
}

#[test]
fn remote_projection_command_sends_typed_intent_without_frontend_io() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let (show_palette, set_show_palette) = signal(true);
        let (is_mobile, _) = signal(false);
        let (sidebar_visible, set_sidebar_visible) = signal(false);
        let (_, set_mobile_visible) = signal(false);
        let (active_view, set_active_view) = signal(SidebarView::Explorer);
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        provide_context(test_session_client(ws.clone(), Some(9)));
        let sidebar_control = SidebarControl {
            is_mobile,
            set_visible: set_sidebar_visible,
            set_mobile_visible,
            set_active_view,
        };
        let commands =
            remote_projection_commands(Locale::En, set_show_palette, None, Some(sidebar_control));
        let pull = commands
            .iter()
            .find(|command| command.id == "webdav:pull")
            .expect("webdav pull command");

        pull.action.run(());

        assert!(!show_palette.get_untracked());
        assert!(sidebar_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
        assert!(matches!(
            ws.drain_sent_for_test().as_slice(),
            [ClientMessage::RemoteProjectionTransport {
                provider: RemoteProjectionProvider::WebDav,
                direction: RemoteProjectionDirection::Pull,
                scope_nonce: Some(9),
            }]
        ));
    });
}

#[test]
fn remote_projection_command_without_session_reports_fail_closed_notice() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let (show_palette, set_show_palette) = signal(true);
        let (notice, set_notice) = signal(None);
        let commands =
            remote_projection_commands(Locale::En, set_show_palette, Some(set_notice), None);
        let s3_pull = commands
            .iter()
            .find(|command| command.id == "s3:pull")
            .expect("s3 pull command");

        s3_pull.action.run(());

        assert!(!show_palette.get_untracked());
        assert!(matches!(
            notice.get_untracked(),
            Some(notice) if crate::hooks::use_core::source_control_notice::is_remote_projection_session_unavailable_notice(&notice)
        ));
    });
}

fn test_session_client(ws: WsService, scope_nonce: Option<u64>) -> SessionClient {
    let (status_text, _) = signal(String::new());
    let (sync_banner, set_sync_banner) = signal(None::<String>);
    let (handshake_ready, _) = signal(scope_nonce.is_some());
    let (handshake_scope_nonce, _) = signal(scope_nonce);
    SessionClient {
        connection_status: ws.status,
        status_text: status_text.into(),
        sync_banner: sync_banner.into(),
        set_sync_banner,
        handshake_ready,
        handshake_scope_nonce,
        on_retry_peer_registration: Callback::new(|_| {}),
        ws,
    }
}

//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 05_diff_logic#remote-projection-transport
//!
//! Web command entries for remote projection transport intents.

use crate::components::activity_bar::SidebarView;
use crate::components::command_palette::types::Command;
use crate::components::main_layout::SidebarControl;
use crate::i18n::{Locale, t};
use crate::runtime::session_client::SessionClient;
use deve_core::protocol::{ClientMessage, RemoteProjectionDirection, RemoteProjectionProvider};
use leptos::prelude::*;

pub(super) fn remote_projection_commands(
    locale: Locale,
    set_show: WriteSignal<bool>,
    sidebar_control: Option<SidebarControl>,
) -> Vec<Command> {
    let enabled_when = (t::command_palette::remote_projection_backend_intent)(locale);
    let group = (t::command_palette::group_remote_projection)(locale);
    let session = use_context::<SessionClient>();
    vec![
        remote_projection_command(
            "webdav:push",
            (t::command_palette::webdav_push)(locale),
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Push,
            enabled_when,
            group,
            set_show,
            sidebar_control,
            session.clone(),
        ),
        remote_projection_command(
            "webdav:pull",
            (t::command_palette::webdav_pull)(locale),
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Pull,
            enabled_when,
            group,
            set_show,
            sidebar_control,
            session.clone(),
        ),
        remote_projection_command(
            "s3:push",
            (t::command_palette::s3_push)(locale),
            RemoteProjectionProvider::S3,
            RemoteProjectionDirection::Push,
            enabled_when,
            group,
            set_show,
            sidebar_control,
            session.clone(),
        ),
        remote_projection_command(
            "s3:pull",
            (t::command_palette::s3_pull)(locale),
            RemoteProjectionProvider::S3,
            RemoteProjectionDirection::Pull,
            enabled_when,
            group,
            set_show,
            sidebar_control,
            session.clone(),
        ),
    ]
}

fn remote_projection_command(
    id: &'static str,
    title: &'static str,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
    enabled_when: &'static str,
    group: &'static str,
    set_show: WriteSignal<bool>,
    sidebar_control: Option<SidebarControl>,
    session: Option<SessionClient>,
) -> Command {
    Command::available(id, title, move || {
        send_remote_projection_transport_intent(session.as_ref(), provider, direction);
        show_source_control_surface(sidebar_control, set_show);
    })
    .with_group(group)
    .with_enabled_when(enabled_when)
}

fn send_remote_projection_transport_intent(
    session: Option<&SessionClient>,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
) {
    let Some(session) = session else {
        return;
    };
    session.ws.send(ClientMessage::RemoteProjectionTransport {
        provider,
        direction,
        scope_nonce: session.handshake_scope_nonce.get_untracked(),
    });
}

fn show_source_control_surface(
    sidebar_control: Option<SidebarControl>,
    set_show: WriteSignal<bool>,
) {
    if let Some(sidebar_control) = sidebar_control {
        sidebar_control.show_view(SidebarView::SourceControl);
    }
    set_show.set(false);
}

#[cfg(test)]
mod tests {
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
            let commands = remote_projection_commands(Locale::En, set_show, None);
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
            assert!(
                commands[1]
                    .detail_text()
                    .contains("provider_io_ready=false")
            );
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
                remote_projection_commands(Locale::En, set_show_palette, Some(sidebar_control));
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
}

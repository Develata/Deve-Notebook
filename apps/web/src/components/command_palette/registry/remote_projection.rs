//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 05_diff_logic#remote-projection-transport
//!
//! Web command entries for remote projection transport intents.

use crate::components::activity_bar::SidebarView;
use crate::components::command_palette::types::Command;
use crate::components::main_layout::SidebarControl;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::i18n::{Locale, t};
use crate::runtime::session_client::SessionClient;
use deve_core::protocol::{ClientMessage, RemoteProjectionDirection, RemoteProjectionProvider};
use leptos::prelude::*;

pub(super) fn remote_projection_commands(
    locale: Locale,
    set_show: WriteSignal<bool>,
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
) -> Vec<Command> {
    let enabled_when = (t::command_palette::remote_projection_backend_intent)(locale);
    let group = (t::command_palette::group_remote_projection)(locale);
    let session = use_context::<SessionClient>();
    let context = RemoteProjectionCommandContext {
        enabled_when,
        group,
        set_show,
        set_notice,
        sidebar_control,
        session,
    };
    vec![
        remote_projection_command(
            "webdav:push",
            (t::command_palette::webdav_push)(locale),
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Push,
            context.clone(),
        ),
        remote_projection_command(
            "webdav:pull",
            (t::command_palette::webdav_pull)(locale),
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Pull,
            context.clone(),
        ),
        remote_projection_command(
            "s3:push",
            (t::command_palette::s3_push)(locale),
            RemoteProjectionProvider::S3,
            RemoteProjectionDirection::Push,
            context.clone(),
        ),
        remote_projection_command(
            "s3:pull",
            (t::command_palette::s3_pull)(locale),
            RemoteProjectionProvider::S3,
            RemoteProjectionDirection::Pull,
            context,
        ),
    ]
}

#[derive(Clone)]
struct RemoteProjectionCommandContext {
    enabled_when: &'static str,
    group: &'static str,
    set_show: WriteSignal<bool>,
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
    session: Option<SessionClient>,
}

fn remote_projection_command(
    id: &'static str,
    title: &'static str,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
    context: RemoteProjectionCommandContext,
) -> Command {
    let group = context.group;
    let enabled_when = context.enabled_when;
    Command::available(id, title, move || {
        send_remote_projection_transport_intent(
            context.session.as_ref(),
            context.set_notice,
            provider,
            direction,
        );
        show_source_control_surface(context.sidebar_control, context.set_show);
    })
    .with_group(group)
    .with_enabled_when(enabled_when)
}

fn send_remote_projection_transport_intent(
    session: Option<&SessionClient>,
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
) {
    let Some(session) = session else {
        if let Some(set_notice) = set_notice {
            set_notice.set(Some(
                SourceControlNotice::remote_projection_session_unavailable(),
            ));
        }
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
mod tests;

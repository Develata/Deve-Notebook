//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
use crate::api::WsService;
use deve_core::protocol::ServerMessage;
use leptos::prelude::Callback;

use super::super::state::CoreSignals;
use super::message_dispatch_route_control::route_control_message;
use super::message_dispatch_route_projection::route_projection_and_sync_message;
use super::message_dispatch_route_protocol::route_protocol_and_write_message;
use super::message_dispatch_route_runtime::route_runtime_message;
use super::message_sync_dispatch::handle_sc_or_remaining;
use crate::runtime::remote_import_client::RemoteImportClient;
use crate::runtime::repo_control_client::RepoControlClient;

#[derive(Clone, Copy)]
pub(super) struct MessageDispatchClients<'a> {
    pub repo_control: &'a RepoControlClient,
    pub remote_import: &'a RemoteImportClient,
}

pub fn handle_message(
    msg: ServerMessage,
    ws: &WsService,
    signals: CoreSignals,
    locale: crate::i18n::Locale,
    schedule_refresh: &dyn Fn(),
    external_changes_refresh: Callback<()>,
    clients: MessageDispatchClients<'_>,
) {
    let Some(msg) = route_projection_and_sync_message(msg, signals) else {
        return;
    };
    let Some(msg) = route_runtime_message(msg, ws, locale, signals, clients.remote_import) else {
        return;
    };
    let Some(msg) = route_control_message(msg, ws, signals, locale, clients.repo_control) else {
        return;
    };
    let Some(msg) = route_protocol_and_write_message(msg, ws, locale, signals) else {
        return;
    };
    handle_sc_or_remaining(msg, ws, signals, schedule_refresh, external_changes_refresh);
}

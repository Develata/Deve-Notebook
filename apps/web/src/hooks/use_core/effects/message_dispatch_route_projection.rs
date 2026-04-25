use deve_core::protocol::ServerMessage;

use super::super::state::CoreSignals;
#[path = "message_dispatch_route_projection_doc.rs"]
mod doc;
#[path = "message_dispatch_route_projection_sync.rs"]
mod sync;

pub fn route_projection_and_sync_message(
    msg: ServerMessage,
    signals: CoreSignals,
) -> Option<ServerMessage> {
    doc::route_projection_doc_message(msg, signals)
        .and_then(|msg| sync::route_projection_sync_message(msg, signals))
}

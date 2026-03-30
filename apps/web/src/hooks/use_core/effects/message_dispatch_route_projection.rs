use deve_core::protocol::ServerMessage;

use super::super::state::CoreSignals;
#[path = "message_dispatch_route_projection_doc.rs"]
mod doc;
#[path = "message_dispatch_route_projection_sync.rs"]
mod sync;

pub fn route_projection_and_sync_message(
    msg: ServerMessage,
    signals: CoreSignals,
) -> Result<(), ServerMessage> {
    match doc::route_projection_doc_message(msg, signals) {
        Ok(()) => Ok(()),
        Err(msg) => sync::route_projection_sync_message(msg, signals),
    }
}

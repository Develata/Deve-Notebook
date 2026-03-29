use crate::api::WsService;
use crate::i18n::Locale;
use deve_core::protocol::ServerMessage;

use super::super::state::CoreSignals;
use super::message_dispatch_protocol::{
    handle_edit_rejected_message, handle_protocol_error_message,
};
use super::message_dispatch_write::{handle_ack_message, handle_write_ready_message};

pub fn route_protocol_and_write_message(
    msg: ServerMessage,
    ws: &WsService,
    locale: Locale,
    signals: CoreSignals,
) -> Result<(), ServerMessage> {
    match msg {
        ServerMessage::EditRejected { scope_nonce, error } => {
            handle_edit_rejected_message(scope_nonce, error, ws, locale, signals);
            Ok(())
        }
        ServerMessage::ProtocolError {
            error,
            switch_nonce,
            scope_nonce,
        } => {
            handle_protocol_error_message(error, switch_nonce, scope_nonce, ws, locale, signals);
            Ok(())
        }
        ServerMessage::WriteReady {
            peer_id,
            repo_id,
            scope_nonce,
            branch,
        } => {
            handle_write_ready_message(peer_id, repo_id, scope_nonce, branch, ws, signals);
            Ok(())
        }
        ServerMessage::Ack {
            repo_id,
            branch,
            scope_nonce,
            doc_id,
            client_op_id,
            ..
        } => {
            handle_ack_message(repo_id, branch, scope_nonce, doc_id, client_op_id, signals);
            Ok(())
        }
        other => Err(other),
    }
}

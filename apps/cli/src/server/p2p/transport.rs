//! plan_ref:
//!   - 07_network#full-peer-ws-admission

use anyhow::{Context, Result, anyhow};
use deve_core::protocol::ServerMessage;
use deve_core::protocol::frame::decode_server_binary;
use futures::SinkExt;
use tokio_tungstenite::tungstenite::Message;

pub(super) async fn handle_transport_control_frame<S>(
    socket: &mut S,
    frame: Message,
) -> Result<Option<Message>>
where
    S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    match frame {
        Message::Ping(payload) => {
            socket
                .send(Message::Pong(payload))
                .await
                .context("Failed to send P2P Pong frame")?;
            Ok(None)
        }
        Message::Pong(_) => Ok(None),
        other => Ok(Some(other)),
    }
}

pub(super) fn decode_server_message(frame: Message) -> Result<ServerMessage> {
    match frame {
        Message::Binary(bytes) => decode_server_binary(bytes.as_ref()).map_err(|err| anyhow!(err)),
        Message::Text(_) => Err(anyhow!(
            "P2P FullPeer connector requires versioned binary server frames"
        )),
        Message::Ping(_) | Message::Pong(_) => Err(anyhow!("unexpected P2P control frame")),
        Message::Close(_) => Err(anyhow!("P2P peer closed websocket")),
        other => Err(anyhow!("unsupported P2P websocket frame: {other:?}")),
    }
}

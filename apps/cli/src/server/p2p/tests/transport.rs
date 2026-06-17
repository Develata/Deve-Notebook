use super::super::transport::decode_server_message;
use deve_core::protocol::ServerMessage;
use deve_core::protocol::frame::encode_server_binary;
use tokio_tungstenite::tungstenite::Message;

#[test]
fn p2p_transport_accepts_versioned_binary_server_frames() -> anyhow::Result<()> {
    let frame = Message::Binary(encode_server_binary(&ServerMessage::Pong)?);

    let message = decode_server_message(frame)?;

    assert!(matches!(message, ServerMessage::Pong));
    Ok(())
}

#[test]
fn p2p_transport_rejects_text_json_server_frames() -> anyhow::Result<()> {
    let text = serde_json::to_string(&ServerMessage::Pong)?;

    let err = decode_server_message(Message::Text(text.into()))
        .expect_err("FullPeer connector must reject text server frames");

    assert!(err.to_string().contains("versioned binary"));
    Ok(())
}

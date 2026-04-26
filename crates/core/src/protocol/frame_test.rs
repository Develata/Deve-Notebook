use super::*;

#[test]
fn client_binary_frame_roundtrips() {
    let bytes = encode_client_binary(&ClientMessage::Ping).unwrap();
    assert!(bytes.starts_with(WS_FRAME_MAGIC));
    assert!(matches!(
        decode_client_binary(&bytes),
        Ok(ClientMessage::Ping)
    ));
}

#[test]
fn server_binary_frame_roundtrips() {
    let bytes = encode_server_binary(&ServerMessage::Pong).unwrap();
    assert!(matches!(
        decode_server_binary(&bytes),
        Ok(ServerMessage::Pong)
    ));
}

#[test]
fn binary_decode_reports_versioned_binary_format() {
    let bytes = encode_client_binary(&ClientMessage::Ping).unwrap();
    let decoded = decode_client_binary_with_format(&bytes).unwrap();

    assert_eq!(decoded.format, WsFrameFormat::VersionedBinary);
    assert!(matches!(decoded.message, ClientMessage::Ping));
}

#[test]
fn legacy_binary_without_magic_is_rejected() {
    let bytes = bincode::serialize(&ClientMessage::Ping).unwrap();
    assert!(matches!(
        decode_client_binary(&bytes),
        Err(ProtocolFrameError::Decode(_))
    ));
}

#[test]
fn json_frame_reports_versioned_text_format() {
    let frame = serde_json::to_string(&ClientFrame::current(ClientMessage::Ping)).unwrap();
    let decoded = decode_client_json_with_format(&frame).unwrap();

    assert_eq!(decoded.format, WsFrameFormat::VersionedJsonText);
    assert!(matches!(decoded.message, ClientMessage::Ping));
}

#[test]
fn legacy_json_text_remains_debug_compatible() {
    let decoded = decode_client_json_with_format(r#""Ping""#).unwrap();

    assert_eq!(decoded.format, WsFrameFormat::LegacyJsonText);
    assert!(matches!(decoded.message, ClientMessage::Ping));
}

#[test]
fn unsupported_version_is_rejected() {
    let frame = ServerFrame {
        protocol_version: WS_PROTOCOL_VERSION - 1,
        message: ServerMessage::Pong,
    };
    let bytes = encode_binary_frame(&frame).unwrap();
    assert!(matches!(
        decode_server_binary(&bytes),
        Err(ProtocolFrameError::UnsupportedVersion { .. })
    ));
}

//! plan_ref:
//!   - 07_network#web-ws-runtime

use super::*;
use deve_core::protocol::frame::decode_client_binary;

#[test]
fn outbound_admission_is_hard_bounded_and_preserves_fifo() {
    let (sender, mut receiver) = outbound_channel_with_limits(3, 1024);
    sender.try_admit(ClientMessage::Ping).unwrap();
    sender.try_admit(ClientMessage::Ping).unwrap();
    sender.try_admit(ClientMessage::Ping).unwrap();

    assert_eq!(
        sender.try_admit(ClientMessage::Ping).unwrap_err(),
        OutboundAdmissionFailure {
            kind: OutboundAdmissionFailureKind::Saturated,
            message_class: OutboundMessageClass::Keepalive,
        }
    );
    for _ in 0..3 {
        let frame = receiver.try_recv().unwrap();
        assert!(matches!(
            decode_client_binary(frame.bytes()),
            Ok(ClientMessage::Ping)
        ));
    }
    assert!(receiver.try_recv().is_err());
}

#[test]
fn cloned_outbound_admission_handles_do_not_expand_capacity() {
    let (sender, _receiver) = outbound_channel_with_limits(2, 1024);
    let cloned_handle = sender.clone();
    sender.try_admit(ClientMessage::Ping).unwrap();
    cloned_handle.try_admit(ClientMessage::Ping).unwrap();

    assert_eq!(
        sender.try_admit(ClientMessage::Ping).unwrap_err().kind,
        OutboundAdmissionFailureKind::Saturated
    );
}

#[test]
fn outbound_admission_count_budget_survives_channel_drain() {
    let (sender, mut receiver) = outbound_channel_with_limits(2, 1024);
    sender.try_admit(ClientMessage::Ping).unwrap();
    sender.try_admit(ClientMessage::Ping).unwrap();
    let retained_frames = [receiver.try_recv().unwrap(), receiver.try_recv().unwrap()];

    assert_eq!(
        sender.try_admit(ClientMessage::Ping).unwrap_err().kind,
        OutboundAdmissionFailureKind::Saturated
    );
    drop(retained_frames);
    sender.try_admit(ClientMessage::Ping).unwrap();
}

#[test]
fn outbound_admission_byte_budget_survives_channel_drain() {
    let ping_bytes = encode_client_binary(&ClientMessage::Ping).unwrap().len();
    let (sender, mut receiver) = outbound_channel_with_limits(8, ping_bytes);
    sender.try_admit(ClientMessage::Ping).unwrap();
    let retained_frame = receiver.try_recv().unwrap();

    assert_eq!(
        sender.try_admit(ClientMessage::Ping).unwrap_err().kind,
        OutboundAdmissionFailureKind::Saturated
    );
    drop(retained_frame);
    sender.try_admit(ClientMessage::Ping).unwrap();
}

#[test]
fn outbound_admission_reports_closed_without_message_payload() {
    let (sender, receiver) = outbound_channel_with_limits(1, 1024);
    drop(receiver);

    let failure = sender.try_admit(ClientMessage::Ping).unwrap_err();
    assert_eq!(failure.kind, OutboundAdmissionFailureKind::Closed);
    assert_eq!(failure.message_class, OutboundMessageClass::Keepalive);
    assert_eq!(failure.kind.label(), "closed");
}

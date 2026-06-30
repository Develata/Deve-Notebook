use super::{
    ApplyEditPlanError, build_append_markdown_op, build_append_markdown_op_at_utf16_len,
    build_apply_edit_message,
};
use deve_core::models::{DocId, Op};
use deve_core::protocol::ClientMessage;

#[test]
fn chat_apply_append_markdown_op_uses_utf16_end_position() {
    assert_eq!(
        build_append_markdown_op("a🙂", " patch".to_string()),
        Ok(Op::Insert {
            pos: 3,
            content: " patch".into(),
        })
    );
}

#[test]
fn chat_apply_append_markdown_op_fails_closed_when_position_overflows() {
    assert_eq!(
        build_append_markdown_op_at_utf16_len(u32::MAX as usize + 1, " patch".to_string()),
        Err(ApplyEditPlanError::DocumentTooLarge)
    );
}

#[test]
fn chat_apply_edit_message_carries_current_scope_nonce() {
    let doc_id = DocId::from_u128(7);
    let op = Op::Insert {
        pos: 3,
        content: " patch".into(),
    };

    match build_apply_edit_message(doc_id, op.clone(), 11, 13, 17) {
        ClientMessage::Edit {
            doc_id: actual_doc_id,
            op: actual_op,
            client_id,
            client_op_id,
            scope_nonce,
        } => {
            assert_eq!(actual_doc_id, doc_id);
            assert_eq!(actual_op, op);
            assert_eq!(client_id, 11);
            assert_eq!(client_op_id, 13);
            assert_eq!(scope_nonce, Some(17));
        }
        other => panic!("expected Edit message, got {other:?}"),
    }
}

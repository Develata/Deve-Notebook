//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 09_web_thin_client_ledger#web-edit-intent

use crate::models::Op;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientOrigin {
    pub client_id: u64,
    pub client_op_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmedOp {
    pub seq: u64,
    pub op: Op,
    pub origin: Option<ClientOrigin>,
}

impl ConfirmedOp {
    pub fn new(seq: u64, op: Op, origin: Option<ClientOrigin>) -> Self {
        Self { seq, op, origin }
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientOrigin, ConfirmedOp};
    use crate::models::Op;

    #[test]
    fn bincode_roundtrip_preserves_none_origin() {
        let op = ConfirmedOp::new(
            7,
            Op::Insert {
                pos: 0,
                content: "hello".into(),
            },
            None,
        );
        let bytes = bincode::serialize(&op).expect("serialize confirmed op");
        let decoded = bincode::deserialize::<ConfirmedOp>(&bytes).expect("decode confirmed op");
        assert_eq!(decoded, op);
    }

    #[test]
    fn bincode_roundtrip_preserves_some_origin() {
        let op = ConfirmedOp::new(
            9,
            Op::Delete { pos: 3, len: 2 },
            Some(ClientOrigin {
                client_id: 11,
                client_op_id: 13,
            }),
        );
        let bytes = bincode::serialize(&op).expect("serialize confirmed op");
        let decoded = bincode::deserialize::<ConfirmedOp>(&bytes).expect("decode confirmed op");
        assert_eq!(decoded, op);
    }
}

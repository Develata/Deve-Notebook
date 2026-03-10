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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<ClientOrigin>,
}

impl ConfirmedOp {
    pub fn new(seq: u64, op: Op, origin: Option<ClientOrigin>) -> Self {
        Self { seq, op, origin }
    }
}

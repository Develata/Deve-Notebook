//! plan_ref:
//!   - 07_network#full-peer-mesh-v1

use deve_core::models::PeerId;

#[derive(Debug, Default)]
pub(in crate::server) struct ExchangeStats {
    pub(in crate::server) saw_hello: bool,
    pub(in crate::server) authenticated_peer_id: Option<PeerId>,
    pub(in crate::server) allowed_export_sources: Vec<PeerId>,
    pub(in crate::server) requested_import_sources: Vec<PeerId>,
    pub(in crate::server) sent_pushes: u64,
    pub(in crate::server) sent_snapshots: u64,
    pub(in crate::server) applied_pushes: u64,
    pub(in crate::server) applied_snapshots: u64,
}

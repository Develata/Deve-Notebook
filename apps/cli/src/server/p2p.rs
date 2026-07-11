//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 07_network#full-peer-ws-admission

mod connect;
mod exchange;
pub(in crate::server) mod fault_injection;
mod hello;
mod source_sets;
mod stats;
mod transfer;
mod transport;
mod validation;

#[cfg(test)]
mod tests;

pub(super) use crate::server::p2p_connector::spawn_mesh_connectors;
pub(super) use connect::connect_peer_once;
pub(super) use stats::ExchangeStats;

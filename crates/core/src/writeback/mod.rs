//! plan_ref:
//!   - 04_storage#projection-contract
//!   - 04_storage#watcher-contract

mod persist_guard;
mod suppressor;

pub(crate) use persist_guard::PersistGuard;
pub(crate) use suppressor::WriteSuppressor;

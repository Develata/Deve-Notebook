//! plan_ref:
//!   - 03_storage/projection#projection-contract
//!   - 03_storage/watcher#watcher-contract

mod persist_guard;
mod suppressor;

pub(crate) use persist_guard::PersistGuard;
pub(crate) use suppressor::WriteSuppressor;

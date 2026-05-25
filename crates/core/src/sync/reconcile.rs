//! plan_ref:
//!   - 03_storage#projection-contract
//!   - 05_diff_logic#authority-diff-core
//!
//! Compatibility surface for callers that historically imported reconcile from
//! sync. The implementation lives under ledger authority.

pub use crate::ledger::reconcile::*;

#[cfg(test)]
mod tests;

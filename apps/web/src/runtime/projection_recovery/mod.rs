//! plan_ref:
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#projection-recovery-coordinator
//!
//! Projection recovery coordination for the Web thin client. This module
//! consumes backend-provided refresh plans; it never infers authority state.

mod editor_cycle;
mod refresh_cycle;
pub(crate) mod scope;

pub use editor_cycle::{ProjectionRecoveryCoordinator, RecoveryCompletion, RecoveryStart};
pub use refresh_cycle::{
    ProjectionRefreshCoordinator, ProjectionRefreshResponse, ProjectionRefreshScope,
    ProjectionRefreshWork,
};
pub use scope::{ProjectionRecoveryScope, evaluate_recovery};

#[cfg(test)]
mod tests;

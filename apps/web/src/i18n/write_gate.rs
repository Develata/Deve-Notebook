//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 09_web_thin_client_ledger#write-readiness
//!
//! User-visible write-gate banner strings.

mod actions;
mod reasons;
mod templates;

pub use actions::{WriteGateAction, action_label};
pub use reasons::{WriteGateReason, reason_label};
pub use templates::{cannot_action, cannot_send};

#[cfg(test)]
mod tests;

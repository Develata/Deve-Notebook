//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! Context action descriptors shared by Web action surfaces.

mod catalog;
mod intent;
mod projection;
mod resolver;
mod target;
mod types;

pub use intent::{ContextActionIntent, ContextActionResolveRequest};
pub use projection::{ContextActionProjectionRequest, project_context_actions};
pub use resolver::resolve_context_action;
pub use target::ContextActionTarget;
pub use types::{ContextActionIcon, ContextActionId, ContextActionSurface};

#[cfg(test)]
mod tests;

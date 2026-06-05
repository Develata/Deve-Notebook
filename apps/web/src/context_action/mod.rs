//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! Context action descriptors shared by Web action surfaces.

mod catalog;
mod projection;
mod target;
mod types;

pub use catalog::context_action_by_id;
pub use projection::{ContextActionProjectionRequest, project_context_actions};
pub use target::ContextActionTarget;
pub use types::{ContextActionIcon, ContextActionId, ContextActionSurface};

#[cfg(test)]
mod tests;

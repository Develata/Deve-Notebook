//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! Context Action projection from catalog metadata to a concrete Web surface.

use super::catalog::CONTEXT_ACTIONS;
use super::target::ContextActionTarget;
use super::types::{ContextActionDescriptor, ContextActionSurface};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextActionProjectionRequest {
    pub surface: ContextActionSurface,
    pub target: ContextActionTarget,
    pub readonly: bool,
}

impl ContextActionProjectionRequest {
    pub fn new(surface: ContextActionSurface, target: ContextActionTarget, readonly: bool) -> Self {
        Self {
            surface,
            target,
            readonly,
        }
    }
}

pub(crate) fn project_context_actions_from_catalog(
    actions: &[ContextActionDescriptor],
    request: &ContextActionProjectionRequest,
) -> Vec<ContextActionDescriptor> {
    actions
        .iter()
        .copied()
        .filter(|action| action.supports_surface(request.surface))
        .filter(|action| action.is_web_projectable())
        .filter(|action| action.target_kind.accepts(request.target.kind))
        .filter(|action| !request.readonly || action.readonly_allowed)
        .collect()
}

pub fn project_context_actions(
    request: ContextActionProjectionRequest,
) -> Vec<ContextActionDescriptor> {
    project_context_actions_from_catalog(CONTEXT_ACTIONS, &request)
}

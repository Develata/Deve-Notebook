//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! Context Action projection from catalog metadata to a concrete Web surface.

use super::catalog::CONTEXT_ACTIONS;
use super::intent::{ContextActionIntent, ContextActionResolveRequest, ProjectedContextAction};
use super::readiness::ContextActionReadiness;
use super::resolver::context_action_descriptor_resolves;
use super::target::ContextActionTarget;
use super::types::{ContextActionDescriptor, ContextActionSurface};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextActionProjectionRequest {
    pub surface: ContextActionSurface,
    pub target: ContextActionTarget,
    pub readiness: ContextActionReadiness,
}

impl ContextActionProjectionRequest {
    pub fn new(
        surface: ContextActionSurface,
        target: ContextActionTarget,
        readiness: ContextActionReadiness,
    ) -> Self {
        Self {
            surface,
            target,
            readiness,
        }
    }
}

pub(crate) fn project_context_actions_from_catalog(
    actions: &[ContextActionDescriptor],
    request: &ContextActionProjectionRequest,
) -> Vec<ProjectedContextAction> {
    actions
        .iter()
        .copied()
        .filter_map(|descriptor| {
            let intent = ContextActionIntent::with_scope(
                descriptor.id,
                request.surface,
                request.target.clone(),
                request.readiness.scope.clone(),
            );
            let resolve_request =
                ContextActionResolveRequest::new(intent.clone(), request.readiness.clone());

            context_action_descriptor_resolves(descriptor, &resolve_request)
                .then(|| ProjectedContextAction::new(descriptor, intent))
        })
        .collect()
}

pub fn project_context_actions(
    request: ContextActionProjectionRequest,
) -> Vec<ProjectedContextAction> {
    project_context_actions_from_catalog(CONTEXT_ACTIONS, &request)
}

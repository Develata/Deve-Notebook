//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! Context action intents and resolver outputs.

use super::readiness::{ContextActionReadiness, ContextActionScope};
use super::target::ContextActionTarget;
use super::types::{ContextActionDescriptor, ContextActionId, ContextActionSurface};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextActionIntent {
    pub action_id: ContextActionId,
    pub surface: ContextActionSurface,
    pub target: ContextActionTarget,
    pub scope: ContextActionScope,
}

impl ContextActionIntent {
    #[cfg(test)]
    pub fn new(
        action_id: ContextActionId,
        surface: ContextActionSurface,
        target: ContextActionTarget,
    ) -> Self {
        Self::with_scope(action_id, surface, target, ContextActionScope::default())
    }

    pub fn with_scope(
        action_id: ContextActionId,
        surface: ContextActionSurface,
        target: ContextActionTarget,
        scope: ContextActionScope,
    ) -> Self {
        Self {
            action_id,
            surface,
            target,
            scope,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextActionResolveRequest {
    pub intent: ContextActionIntent,
    pub readiness: ContextActionReadiness,
}

impl ContextActionResolveRequest {
    pub fn new(intent: ContextActionIntent, readiness: ContextActionReadiness) -> Self {
        Self { intent, readiness }
    }
}

#[derive(Clone)]
pub struct ProjectedContextAction {
    pub descriptor: ContextActionDescriptor,
    pub intent: ContextActionIntent,
}

impl ProjectedContextAction {
    pub fn new(descriptor: ContextActionDescriptor, intent: ContextActionIntent) -> Self {
        Self { descriptor, intent }
    }
}

#[derive(Clone)]
pub struct ResolvedContextAction {
    pub descriptor: ContextActionDescriptor,
    pub intent: ContextActionIntent,
}

impl ResolvedContextAction {
    pub fn new(descriptor: ContextActionDescriptor, intent: ContextActionIntent) -> Self {
        Self { descriptor, intent }
    }
}

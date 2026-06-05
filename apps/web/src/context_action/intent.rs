//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! Context action intents and resolver outputs.

use super::target::ContextActionTarget;
use super::types::{ContextActionDescriptor, ContextActionId, ContextActionSurface};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextActionIntent {
    pub action_id: ContextActionId,
    pub surface: ContextActionSurface,
    pub target: ContextActionTarget,
}

impl ContextActionIntent {
    pub fn new(
        action_id: ContextActionId,
        surface: ContextActionSurface,
        target: ContextActionTarget,
    ) -> Self {
        Self {
            action_id,
            surface,
            target,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextActionResolveRequest {
    pub intent: ContextActionIntent,
    pub readonly: bool,
}

impl ContextActionResolveRequest {
    pub fn new(intent: ContextActionIntent, readonly: bool) -> Self {
        Self { intent, readonly }
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

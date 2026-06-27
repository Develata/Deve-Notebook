//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! Context action availability resolver for Web flow coordination.

use super::catalog::CONTEXT_ACTIONS;
use super::intent::{ContextActionResolveRequest, ResolvedContextAction};
use super::types::{ContextActionDescriptor, ContextActionEffect};

fn descriptor_matches_request(
    descriptor: ContextActionDescriptor,
    request: &ContextActionResolveRequest,
) -> bool {
    descriptor.supports_surface(request.intent.surface)
        && descriptor.is_web_projectable()
        && descriptor.target_kind.accepts(request.intent.target.kind)
        && request.intent.target.is_repo_user_path()
        && request.intent.scope == request.readiness.scope
        && (!request.readiness.readonly || descriptor.readonly_allowed)
        && (!request.readiness.write_blocked || !is_write_effect(descriptor.effect))
}

fn is_write_effect(effect: ContextActionEffect) -> bool {
    matches!(
        effect,
        ContextActionEffect::AuthorityWrite
            | ContextActionEffect::DestructiveWrite
            | ContextActionEffect::ExternalSideEffect
    )
}

pub(crate) fn context_action_descriptor_resolves(
    descriptor: ContextActionDescriptor,
    request: &ContextActionResolveRequest,
) -> bool {
    descriptor_matches_request(descriptor, request)
}

pub(crate) fn resolve_context_action_from_catalog(
    actions: &[ContextActionDescriptor],
    request: ContextActionResolveRequest,
) -> Option<ResolvedContextAction> {
    let descriptor = actions
        .iter()
        .copied()
        .find(|action| action.id == request.intent.action_id)?;

    if !descriptor_matches_request(descriptor, &request) {
        return None;
    }

    Some(ResolvedContextAction::new(descriptor, request.intent))
}

pub fn resolve_context_action(
    request: ContextActionResolveRequest,
) -> Option<ResolvedContextAction> {
    resolve_context_action_from_catalog(CONTEXT_ACTIONS, request)
}

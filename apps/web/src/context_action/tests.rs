use super::catalog::{CONTEXT_ACTIONS, context_action_by_id};
use super::projection::{
    ContextActionProjectionRequest, project_context_actions, project_context_actions_from_catalog,
};
use super::target::{ContextActionTarget, ContextActionTargetKind};
use super::types::{
    ContextActionDescriptor, ContextActionEffect, ContextActionIcon, ContextActionId,
    ContextActionOrigin, ContextActionSurface,
};

const TEST_SURFACE: &[ContextActionSurface] = &[ContextActionSurface::FileTree];

fn file_tree_request(
    readonly: bool,
    target_kind: ContextActionTargetKind,
) -> ContextActionProjectionRequest {
    ContextActionProjectionRequest::new(
        ContextActionSurface::FileTree,
        ContextActionTarget::new(target_kind, "notes/readme.md"),
        readonly,
    )
}

mod catalog;
mod projection;
mod target;

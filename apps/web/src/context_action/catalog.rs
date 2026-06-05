//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! Static Context Action catalog.

use super::target::ContextActionTargetKind;
use super::types::{
    ContextActionDescriptor, ContextActionEffect, ContextActionIcon, ContextActionId,
    ContextActionOrigin, ContextActionSurface,
};
use crate::i18n::t;

const FILE_TREE_SURFACE: &[ContextActionSurface] = &[ContextActionSurface::FileTree];

pub const CONTEXT_ACTIONS: &[ContextActionDescriptor] = &[
    ContextActionDescriptor {
        id: ContextActionId::Rename,
        label: t::context_menu::rename,
        icon: ContextActionIcon::Rename,
        origin: ContextActionOrigin::BackendNativeIntent,
        target_kind: ContextActionTargetKind::AnyNode,
        effect: ContextActionEffect::AuthorityWrite,
        readonly_allowed: false,
        separator_before: false,
        surfaces: FILE_TREE_SURFACE,
    },
    ContextActionDescriptor {
        id: ContextActionId::Copy,
        label: t::context_menu::copy,
        icon: ContextActionIcon::Copy,
        origin: ContextActionOrigin::BackendNativeIntent,
        target_kind: ContextActionTargetKind::AnyNode,
        effect: ContextActionEffect::AuthorityWrite,
        readonly_allowed: false,
        separator_before: false,
        surfaces: FILE_TREE_SURFACE,
    },
    ContextActionDescriptor {
        id: ContextActionId::OpenInNewWindow,
        label: t::context_menu::open_in_new_window,
        icon: ContextActionIcon::OpenInNewWindow,
        origin: ContextActionOrigin::ShellLocal,
        target_kind: ContextActionTargetKind::AnyNode,
        effect: ContextActionEffect::ReadOnly,
        readonly_allowed: true,
        separator_before: false,
        surfaces: FILE_TREE_SURFACE,
    },
    ContextActionDescriptor {
        id: ContextActionId::MoveTo,
        label: t::context_menu::move_to,
        icon: ContextActionIcon::MoveTo,
        origin: ContextActionOrigin::BackendNativeIntent,
        target_kind: ContextActionTargetKind::AnyNode,
        effect: ContextActionEffect::AuthorityWrite,
        readonly_allowed: false,
        separator_before: true,
        surfaces: FILE_TREE_SURFACE,
    },
    ContextActionDescriptor {
        id: ContextActionId::Delete,
        label: t::context_menu::delete,
        icon: ContextActionIcon::Delete,
        origin: ContextActionOrigin::BackendNativeIntent,
        target_kind: ContextActionTargetKind::AnyNode,
        effect: ContextActionEffect::DestructiveWrite,
        readonly_allowed: false,
        separator_before: true,
        surfaces: FILE_TREE_SURFACE,
    },
    ContextActionDescriptor {
        id: ContextActionId::ExportPdf,
        label: t::context_menu::export_pdf,
        icon: ContextActionIcon::ExportPdf,
        origin: ContextActionOrigin::ExternalProcess,
        target_kind: ContextActionTargetKind::MarkdownFile,
        effect: ContextActionEffect::ExternalSideEffect,
        readonly_allowed: true,
        separator_before: true,
        surfaces: FILE_TREE_SURFACE,
    },
];

#[cfg(test)]
pub(crate) fn context_action_by_id(id: ContextActionId) -> Option<ContextActionDescriptor> {
    CONTEXT_ACTIONS
        .iter()
        .copied()
        .find(|action| action.id == id)
}

//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! Core Context Action metadata types.

use super::target::ContextActionTargetKind;
use crate::i18n::Locale;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextActionId {
    Rename,
    Copy,
    OpenInNewWindow,
    CopyAbsolutePath,
    RevealInSystemExplorer,
    MoveTo,
    Delete,
    ExportPdf,
}

impl ContextActionId {
    pub fn stable_id(self) -> &'static str {
        match self {
            Self::Rename => "file.rename",
            Self::Copy => "file.copy",
            Self::OpenInNewWindow => "file.open_in_new_window",
            Self::CopyAbsolutePath => "file.copy_absolute_path",
            Self::RevealInSystemExplorer => "file.reveal_in_system_explorer",
            Self::MoveTo => "file.move_to",
            Self::Delete => "file.delete",
            Self::ExportPdf => "file.export_pdf",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextActionSurface {
    FileTree,
    #[allow(dead_code)]
    CommandPalette,
    #[allow(dead_code)]
    Shortcut,
    #[allow(dead_code)]
    Toolbar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextActionOrigin {
    ShellLocal,
    BackendNativeIntent,
    // Reserved for backend-provided descriptors that launch non-native tools.
    ExternalProcess,
}

impl ContextActionOrigin {
    pub fn requires_external_provenance(self) -> bool {
        matches!(self, Self::ExternalProcess)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextActionEffect {
    ReadOnly,
    AuthorityWrite,
    DestructiveWrite,
    // Reserved for external actions whose side effects are outside the Rust authority path.
    ExternalSideEffect,
}

impl ContextActionEffect {
    pub fn is_destructive(self) -> bool {
        matches!(self, Self::DestructiveWrite)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextActionIcon {
    Rename,
    Copy,
    OpenInNewWindow,
    CopyAbsolutePath,
    RevealInSystemExplorer,
    MoveTo,
    Delete,
    ExportPdf,
}

#[derive(Clone, Copy)]
pub struct ContextActionDescriptor {
    pub id: ContextActionId,
    pub label: fn(Locale) -> &'static str,
    pub icon: ContextActionIcon,
    pub origin: ContextActionOrigin,
    pub target_kind: ContextActionTargetKind,
    pub effect: ContextActionEffect,
    pub readonly_allowed: bool,
    pub separator_before: bool,
    pub surfaces: &'static [ContextActionSurface],
}

impl ContextActionDescriptor {
    pub fn stable_id(self) -> &'static str {
        self.id.stable_id()
    }

    pub fn label(self, locale: Locale) -> &'static str {
        (self.label)(locale)
    }

    pub fn is_destructive(self) -> bool {
        self.effect.is_destructive()
    }

    pub fn shows_external_provenance(self) -> bool {
        self.origin.requires_external_provenance()
    }

    pub fn is_web_projectable(self) -> bool {
        !matches!(self.origin, ContextActionOrigin::ExternalProcess)
    }

    pub fn supports_surface(self, surface: ContextActionSurface) -> bool {
        self.surfaces.contains(&surface)
    }
}

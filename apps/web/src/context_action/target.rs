//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!   - 11_ui_design/index#context-action-surface
//!
//! Context Action target normalization and matching.

use deve_core::utils::notegit::is_internal_repo_segment;
use deve_core::utils::path::to_forward_slash;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextActionTargetKind {
    AnyNode,
    File,
    Folder,
    MarkdownFile,
}

impl ContextActionTargetKind {
    pub fn accepts(self, actual: Self) -> bool {
        matches!(
            (self, actual),
            (Self::AnyNode, _)
                | (Self::File, Self::File)
                | (Self::Folder, Self::Folder)
                | (Self::MarkdownFile, Self::MarkdownFile)
                | (Self::File, Self::MarkdownFile)
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextActionTarget {
    pub kind: ContextActionTargetKind,
    pub path: String,
}

impl ContextActionTarget {
    pub fn new(kind: ContextActionTargetKind, path: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            kind,
            path: to_forward_slash(&path),
        }
    }

    pub fn from_file_tree_node(is_folder: bool, path: &str) -> Self {
        let normalized = to_forward_slash(path);
        let kind = if is_folder {
            ContextActionTargetKind::Folder
        } else {
            let extension = normalized
                .rsplit('/')
                .next()
                .and_then(|name| name.rsplit_once('.').map(|(_, ext)| ext));

            match extension {
                Some(ext)
                    if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown") =>
                {
                    ContextActionTargetKind::MarkdownFile
                }
                _ => ContextActionTargetKind::File,
            }
        };

        Self::new(kind, normalized)
    }

    pub(crate) fn is_repo_user_path(&self) -> bool {
        !self
            .path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .any(is_internal_repo_segment)
    }
}

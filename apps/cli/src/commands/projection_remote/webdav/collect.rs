//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!
//! Compatibility shim for WebDAV tests; the shared collector lives one level up
//! because S3 push uses the same Markdown Projection Workspace file set.

#[cfg(test)]
pub(crate) use crate::commands::projection_remote::collect::{
    MarkdownProjectionFileRef, collect_markdown_projection_files,
};

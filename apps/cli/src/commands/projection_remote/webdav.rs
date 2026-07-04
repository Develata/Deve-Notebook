//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#cli-commands
//!
//! CLI WebDAV adapter for Markdown Projection Workspace push.

mod collect;
mod provider;

#[cfg(test)]
pub(crate) use collect::MarkdownProjectionFileRef;
pub(crate) use collect::collect_markdown_projection_files;
pub(crate) use provider::{WebDavProjectionProvider, WebDavProjectionPushAdapter};

#[cfg(test)]
mod tests;

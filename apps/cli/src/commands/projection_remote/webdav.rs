//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#cli-commands
//!
//! CLI WebDAV adapter for Markdown Projection Workspace push/pull.

mod collect;
mod provider;
mod pull;
mod push;
mod transport;
mod url;
mod workspace_apply;

#[cfg(test)]
pub(crate) use collect::MarkdownProjectionFileRef;
pub(crate) use collect::collect_markdown_projection_files;
pub(crate) use provider::WebDavProjectionProvider;
pub(crate) use pull::WebDavProjectionPullAdapter;
pub(crate) use push::WebDavProjectionPushAdapter;

pub(crate) trait WebDavProjectionAdapter:
    WebDavProjectionPushAdapter + WebDavProjectionPullAdapter
{
}

impl<T> WebDavProjectionAdapter for T where
    T: WebDavProjectionPushAdapter + WebDavProjectionPullAdapter
{
}

#[cfg(test)]
mod tests;

//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#cli-commands
//!
//! CLI S3 adapter for Markdown Projection Workspace push.

mod credentials;
mod provider;
mod push;
mod signing;
mod transport;
mod url;

pub(crate) use provider::FailClosedS3ProjectionProvider;
pub(crate) use provider::S3ProjectionProvider;
pub(crate) use push::S3ProjectionPushAdapter;

#[cfg(test)]
mod tests;

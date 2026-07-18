//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract
//!
//! Shared WebDAV adapter for Projection push and ordered source acquisition.

mod provider;
mod push;
mod source;
mod transport;
mod url;

pub(crate) use provider::WebDavProjectionProvider;
pub(crate) use push::WebDavProjectionPushAdapter;

#[cfg(test)]
mod tests;

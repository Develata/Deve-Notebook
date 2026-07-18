//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract

use super::transport::ReqwestWebDavTransport;
use deve_core::remote_projection::RemoteProjectionProviderError;

pub(crate) struct WebDavProjectionProvider<T = ReqwestWebDavTransport> {
    #[cfg(test)]
    pub(super) transport: T,
    #[cfg(not(test))]
    pub(super) transport: T,
}

impl Default for WebDavProjectionProvider<ReqwestWebDavTransport> {
    fn default() -> Self {
        Self::new().expect("reqwest WebDAV transport with static timeout config")
    }
}

impl WebDavProjectionProvider<ReqwestWebDavTransport> {
    pub(crate) fn new() -> Result<Self, RemoteProjectionProviderError> {
        Ok(Self {
            transport: ReqwestWebDavTransport::new()?,
        })
    }
}

#[cfg(test)]
impl<T> WebDavProjectionProvider<T> {
    pub(super) fn new_for_test(transport: T) -> Self {
        Self { transport }
    }
}

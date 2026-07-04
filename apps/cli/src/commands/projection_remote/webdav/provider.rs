//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::pull;
use super::push;
use super::transport::{ReqwestWebDavTransport, WebDavTransport};
use deve_core::remote_projection::{
    RemoteProjectionProvider, RemoteProjectionProviderAdapter, RemoteProjectionProviderError,
    RemoteProjectionPullOutcome, RemoteProjectionPullRequest, RemoteProjectionPushOutcome,
    RemoteProjectionPushRequest,
};

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

impl<T: WebDavTransport> RemoteProjectionProviderAdapter for WebDavProjectionProvider<T> {
    fn provider(&self) -> RemoteProjectionProvider {
        RemoteProjectionProvider::WebDav
    }

    fn push(
        &mut self,
        request: RemoteProjectionPushRequest,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        push::push_request(&self.transport, request)
    }

    fn pull(
        &self,
        request: RemoteProjectionPullRequest,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        pull::pull_request(&self.transport, request)
    }
}

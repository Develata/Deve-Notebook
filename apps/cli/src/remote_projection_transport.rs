//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract
//!   - 06_backup#remote-import-runtime-boundary
//!
//! Shared host runtime for Remote Projection push and immutable remote-source
//! acquisition. This module owns provider/profile/HTTP/signing concerns only;
//! it has no repository, session, Ledger, or workspace-mutation authority.

mod body;
mod contract;
mod path_set;
mod push_error;
mod push_source;
pub(crate) mod s3;
pub(crate) mod webdav;
mod xml;

#[cfg(test)]
pub(crate) use contract::TestCollectingSourceSink;
pub(crate) use contract::{
    ProjectionPushSource, ProjectionPushVisitor, RemoteSourceAcquisition, RemoteSourceSink,
    SourceAcquisitionError, SourceAcquisitionOutcome, SourceAcquisitionRequest,
    TransportCapability, ensure_projection_transport_push_outcome_contract,
};
pub(crate) use path_set::NormalizedRemotePath;
#[cfg(test)]
pub(crate) use path_set::{MAX_SOURCE_FILES, MAX_SOURCE_PATH_BYTES};
pub(crate) use push_error::ProjectionPushError;
pub(crate) use push_source::WorkspaceProjectionPushSource;
#[cfg(test)]
pub(crate) use push_source::collect_markdown_projection_files;

/// Executes provider I/O for a previously admitted push source. Repository
/// identity and workspace selection stay with the caller; the transport
/// runtime owns only provider construction, upload, and outcome validation.
pub(crate) fn push_projection_from_source(
    ledger_dir: &std::path::Path,
    provider: deve_core::remote_projection::RemoteProjectionProvider,
    locator: &str,
    source: &dyn ProjectionPushSource,
) -> Result<deve_core::remote_projection::RemoteProjectionPushOutcome, ProjectionPushError> {
    use s3::S3ProjectionPushAdapter as _;
    use webdav::WebDavProjectionPushAdapter as _;

    let outcome = match provider {
        deve_core::remote_projection::RemoteProjectionProvider::WebDav => {
            let mut adapter = webdav::WebDavProjectionProvider::new()
                .map_err(ProjectionPushError::provider_unavailable)?;
            adapter.push_projection_files(provider, locator, source)
        }
        deve_core::remote_projection::RemoteProjectionProvider::S3 => {
            let (mut adapter, _) =
                s3::provider_for_locator(ledger_dir, TransportCapability::Push, locator)
                    .map_err(ProjectionPushError::provider_unavailable)?;
            adapter.push_projection_files(provider, locator, source)
        }
    }?;
    ensure_projection_transport_push_outcome_contract(&outcome)?;
    Ok(outcome)
}

pub(crate) fn admit_repo_url(
    provider: deve_core::remote_projection::RemoteProjectionProvider,
    _capability: TransportCapability,
    locator: &str,
) -> Result<String, deve_core::remote_projection::RemoteProjectionProviderError> {
    Ok(deve_core::remote_projection::validate_remote_projection_locator(provider, locator)?)
}

#[cfg(test)]
pub(crate) mod redirect_test_support {
    use deve_core::remote_projection::RemoteProjectionProviderError;
    use reqwest::{StatusCode, Url};
    use std::io::{ErrorKind, Read, Write};
    use std::net::TcpListener;

    pub(crate) fn assert_redirect_not_followed(
        send: impl FnOnce(Url) -> Result<StatusCode, RemoteProjectionProviderError>,
    ) {
        let target = TcpListener::bind("127.0.0.1:0").expect("target listener");
        let target_address = target.local_addr().expect("target address");
        let source = TcpListener::bind("127.0.0.1:0").expect("source listener");
        let source_address = source.local_addr().expect("source address");
        let source_thread = std::thread::spawn(move || {
            let (mut stream, _) = source.accept().expect("source request");
            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).expect("read request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            write!(
                stream,
                "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{target_address}/credential-leak\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .expect("redirect response");
        });

        let status = send(
            Url::parse(&format!("http://{source_address}/source")).expect("source request URL"),
        )
        .expect("transport response");
        assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
        source_thread.join().expect("source server");

        target.set_nonblocking(true).expect("nonblocking target");
        match target.accept() {
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Ok(_) => panic!("transport followed a credential-bearing redirect"),
            Err(error) => panic!("target accept failed: {error}"),
        }
    }
}

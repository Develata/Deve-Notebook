//! plan_ref:
//!   - 06_backup#remote-import-runtime-boundary
//!   - 07_network#remote-import-wire-contract
//!
//! Provider selection and deterministic source/locator binding material.

use super::RemoteImportHostError;
use crate::remote_projection_transport::SourceAcquisitionRequest;
use deve_core::protocol::RemoteProjectionProvider;
use deve_core::remote_import::RemoteImportBinding;

pub(super) struct ResolvedRemoteSource {
    pub(super) provider: RemoteProjectionProvider,
    pub(super) locator: String,
    pub(super) source_binding: RemoteImportBinding,
    pub(super) locator_binding: RemoteImportBinding,
    pub(super) s3_provider: Option<crate::remote_projection_transport::s3::S3ProjectionProvider>,
}

pub(super) fn infer_provider(
    locator: &str,
) -> Result<RemoteProjectionProvider, RemoteImportHostError> {
    let webdav = SourceAcquisitionRequest::new(RemoteProjectionProvider::WebDav, locator);
    let s3 = SourceAcquisitionRequest::new(RemoteProjectionProvider::S3, locator);
    match (webdav.is_ok(), s3.is_ok()) {
        (true, false) => Ok(RemoteProjectionProvider::WebDav),
        (false, true) => Ok(RemoteProjectionProvider::S3),
        _ => Err(RemoteImportHostError::Locator(
            "remote projection locator does not select exactly one provider".to_string(),
        )),
    }
}

pub(super) fn canonical_binding_material(
    provider: RemoteProjectionProvider,
    locator: Option<&str>,
    profile_id: Option<&str>,
) -> Vec<u8> {
    let mut material = Vec::new();
    for field in [
        provider.as_str(),
        locator.unwrap_or(""),
        profile_id.unwrap_or(""),
    ] {
        material.extend_from_slice(&(field.len() as u64).to_le_bytes());
        material.extend_from_slice(field.as_bytes());
    }
    material
}

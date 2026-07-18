//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract
//!   - 06_backup#projection-backup-secret-ref-contract
//!
//! Host-local, secret-free S3-compatible Remote Projection profiles.

use super::credentials::S3Credentials;
use super::url::{S3CustomEndpointUrlBinding, custom_locator_origin_bucket_prefix};
use crate::remote_projection_transport::TransportCapability;
use deve_core::remote_projection::RemoteProjectionProviderError;
use deve_core::utils::notegit::host_dir;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const PROFILE_FILE: &str = "remote-projection-s3-profiles.toml";
const PROVIDER_KIND: &str = "s3-compatible";
const ADDRESSING_STYLE_PATH: &str = "path";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemoteProjectionS3Profile {
    pub(crate) profile_id: String,
    pub(crate) provider: String,
    pub(crate) endpoint_origin: String,
    pub(crate) bucket: String,
    pub(crate) allowed_prefix: String,
    pub(crate) region: String,
    #[serde(default = "default_addressing_style")]
    pub(crate) addressing_style: String,
    pub(crate) allowed_capabilities: Vec<String>,
    pub(crate) credential_ref: RemoteProjectionS3CredentialRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemoteProjectionS3CredentialRef {
    pub(crate) env_prefix: String,
}

#[derive(Clone)]
pub(super) struct S3ProfileRuntimeBinding {
    pub(super) credentials: S3Credentials,
    pub(super) region: String,
    pub(super) url_binding: S3CustomEndpointUrlBinding,
}

impl std::fmt::Debug for S3ProfileRuntimeBinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("S3ProfileRuntimeBinding")
            .field("credentials", &self.credentials)
            .field("region", &self.region)
            .field("url_binding", &self.url_binding)
            .finish()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoteProjectionS3ProfileStore {
    #[serde(default)]
    profiles: Vec<RemoteProjectionS3Profile>,
}

impl RemoteProjectionS3Profile {
    pub(crate) fn env_profile(
        profile_id: impl Into<String>,
        endpoint_origin: impl Into<String>,
        bucket: impl Into<String>,
        allowed_prefix: impl Into<String>,
        region: impl Into<String>,
        credential_env_prefix: impl Into<String>,
        allowed_capabilities: Vec<String>,
    ) -> Self {
        Self {
            profile_id: profile_id.into(),
            provider: PROVIDER_KIND.into(),
            endpoint_origin: endpoint_origin.into(),
            bucket: bucket.into(),
            allowed_prefix: allowed_prefix.into(),
            region: region.into(),
            addressing_style: ADDRESSING_STYLE_PATH.into(),
            allowed_capabilities,
            credential_ref: RemoteProjectionS3CredentialRef {
                env_prefix: credential_env_prefix.into(),
            },
        }
    }

    pub(super) fn runtime_binding_for(
        &self,
        capability: TransportCapability,
        locator: &str,
    ) -> Result<S3ProfileRuntimeBinding, RemoteProjectionProviderError> {
        self.ensure_locator_binding(capability, locator)?;
        let endpoint_origin = Url::parse(&normalize_endpoint_origin(&self.endpoint_origin)?)
            .map_err(|err| profile_error(format!("invalid endpoint origin: {err}")))?;
        Ok(S3ProfileRuntimeBinding {
            credentials: self.credentials_from_env_prefix()?,
            region: self.region.trim().to_string(),
            url_binding: S3CustomEndpointUrlBinding::new(endpoint_origin, self.bucket.trim())?,
        })
    }

    pub(super) fn ensure_locator_binding(
        &self,
        capability: TransportCapability,
        locator: &str,
    ) -> Result<(), RemoteProjectionProviderError> {
        self.validate()?;
        self.ensure_capability_allowed(capability)?;
        let (locator_origin, locator_bucket, locator_prefix) =
            custom_locator_origin_bucket_prefix(locator)?;
        if locator_origin != normalize_endpoint_origin(&self.endpoint_origin)? {
            return Err(profile_error(format!(
                "S3 custom endpoint profile {} does not match locator endpoint origin",
                self.profile_id
            )));
        }
        if locator_bucket != self.bucket.trim() {
            return Err(profile_error(format!(
                "S3 custom endpoint profile {} does not match locator bucket",
                self.profile_id
            )));
        }
        let allowed_prefix = normalize_prefix(&self.allowed_prefix)?;
        if !prefix_is_within_allowed_root(&locator_prefix, &allowed_prefix) {
            return Err(profile_error(format!(
                "S3 custom endpoint profile {} does not allow locator prefix",
                self.profile_id
            )));
        }
        Ok(())
    }

    pub(crate) fn validate(&self) -> Result<(), RemoteProjectionProviderError> {
        validate_profile_id(&self.profile_id)?;
        if self.provider.trim() != PROVIDER_KIND {
            return Err(profile_error(format!(
                "Remote Projection S3 profile {} provider must be {PROVIDER_KIND}",
                self.profile_id
            )));
        }
        normalize_endpoint_origin(&self.endpoint_origin)?;
        validate_bucket(&self.bucket)?;
        normalize_prefix(&self.allowed_prefix)?;
        if self.region.trim().is_empty() {
            return Err(profile_error(format!(
                "Remote Projection S3 profile {} region is required",
                self.profile_id
            )));
        }
        if self.addressing_style.trim() != ADDRESSING_STYLE_PATH {
            return Err(profile_error(format!(
                "Remote Projection S3 profile {} addressing_style must be path",
                self.profile_id
            )));
        }
        validate_allowed_capabilities(&self.allowed_capabilities)?;
        validate_env_prefix(&self.credential_ref.env_prefix)?;
        Ok(())
    }

    fn ensure_capability_allowed(
        &self,
        capability: TransportCapability,
    ) -> Result<(), RemoteProjectionProviderError> {
        let wanted = capability.profile_name();
        if self
            .allowed_capabilities
            .iter()
            .any(|entry| entry.trim().eq_ignore_ascii_case(wanted))
        {
            Ok(())
        } else {
            Err(profile_error(format!(
                "S3 custom endpoint profile {} does not allow {}",
                self.profile_id, wanted
            )))
        }
    }

    fn credentials_from_env_prefix(&self) -> Result<S3Credentials, RemoteProjectionProviderError> {
        let prefix = self.credential_ref.env_prefix.trim();
        Ok(S3Credentials {
            access_key_id: required_prefixed_env(prefix, "ACCESS_KEY_ID")?,
            secret_access_key: required_prefixed_env(prefix, "SECRET_ACCESS_KEY")?,
            session_token: optional_prefixed_env(prefix, "SESSION_TOKEN"),
        })
    }
}

pub(crate) fn profile_store_path(ledger_dir: &Path) -> PathBuf {
    host_dir(ledger_dir).join(PROFILE_FILE)
}

pub(crate) fn load_remote_projection_s3_profiles(
    ledger_dir: &Path,
) -> Result<Vec<RemoteProjectionS3Profile>, RemoteProjectionProviderError> {
    let path = profile_store_path(ledger_dir);
    let store = load_store(&path)?;
    validate_store(&store)?;
    Ok(store.profiles)
}

pub(crate) fn load_remote_projection_s3_profile(
    ledger_dir: &Path,
    profile_id: &str,
) -> Result<RemoteProjectionS3Profile, RemoteProjectionProviderError> {
    validate_profile_id(profile_id)?;
    load_remote_projection_s3_profiles(ledger_dir)?
        .into_iter()
        .find(|profile| profile.profile_id == profile_id)
        .ok_or_else(|| {
            profile_error(format!(
                "Remote Projection S3 profile {profile_id} is not configured (provider_io_ready=false)"
            ))
        })
}

pub(crate) fn write_remote_projection_s3_profile(
    ledger_dir: &Path,
    profile: RemoteProjectionS3Profile,
) -> Result<PathBuf, RemoteProjectionProviderError> {
    profile.validate()?;
    let path = profile_store_path(ledger_dir);
    let mut store = load_store(&path)?;
    store
        .profiles
        .retain(|existing| existing.profile_id != profile.profile_id);
    store.profiles.push(profile);
    store
        .profiles
        .sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
    validate_store(&store)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            profile_error(format!(
                "failed to create Remote Projection profile directory {}: {err}",
                parent.display()
            ))
        })?;
    }
    let output = toml::to_string_pretty(&store)
        .map_err(|err| profile_error(format!("failed to render profile store: {err}")))?;
    std::fs::write(&path, output).map_err(|err| {
        profile_error(format!(
            "failed to write Remote Projection profile store {}: {err}",
            path.display()
        ))
    })?;
    Ok(path)
}

fn load_store(
    path: &Path,
) -> Result<RemoteProjectionS3ProfileStore, RemoteProjectionProviderError> {
    match std::fs::read_to_string(path) {
        Ok(input) if input.trim().is_empty() => Ok(RemoteProjectionS3ProfileStore::default()),
        Ok(input) => toml::from_str(&input).map_err(|err| {
            profile_error(format!(
                "failed to parse Remote Projection S3 profile store {}: {err}",
                path.display()
            ))
        }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok(RemoteProjectionS3ProfileStore::default())
        }
        Err(err) => Err(profile_error(format!(
            "failed to read Remote Projection S3 profile store {}: {err}",
            path.display()
        ))),
    }
}

fn validate_store(
    store: &RemoteProjectionS3ProfileStore,
) -> Result<(), RemoteProjectionProviderError> {
    let mut seen = BTreeSet::new();
    for profile in &store.profiles {
        profile.validate()?;
        if !seen.insert(profile.profile_id.as_str()) {
            return Err(profile_error(format!(
                "Remote Projection S3 profile {} is duplicated",
                profile.profile_id
            )));
        }
    }
    Ok(())
}

fn normalize_endpoint_origin(origin: &str) -> Result<String, RemoteProjectionProviderError> {
    let mut url = Url::parse(origin.trim())
        .map_err(|err| profile_error(format!("invalid endpoint origin: {err}")))?;
    if url.scheme() != "https" {
        return Err(profile_error(
            "S3 custom endpoint profile endpoint_origin must use https".to_string(),
        ));
    }
    if url.host_str().is_none() {
        return Err(profile_error(
            "S3 custom endpoint profile endpoint_origin has no host".to_string(),
        ));
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(profile_error(
            "S3 custom endpoint profile endpoint_origin must not contain credentials, query, or fragment data".to_string(),
        ));
    }
    let path = url.path();
    if !(path.is_empty() || path == "/") {
        return Err(profile_error(
            "S3 custom endpoint profile endpoint_origin must be an origin without a path"
                .to_string(),
        ));
    }
    url.set_path("");
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn validate_profile_id(profile_id: &str) -> Result<(), RemoteProjectionProviderError> {
    let profile_id = profile_id.trim();
    if profile_id.is_empty() {
        return Err(profile_error("Remote Projection S3 profile_id is required"));
    }
    if profile_id.len() > 64
        || !profile_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(profile_error(
            "Remote Projection S3 profile_id must use only ASCII letters, digits, '-' or '_'",
        ));
    }
    Ok(())
}

fn validate_bucket(bucket: &str) -> Result<(), RemoteProjectionProviderError> {
    let bucket = bucket.trim();
    if bucket.is_empty() || bucket.contains('/') || bucket.contains(':') {
        return Err(profile_error(
            "S3 custom endpoint profile bucket must be a single bucket name segment",
        ));
    }
    Ok(())
}

fn normalize_prefix(prefix: &str) -> Result<String, RemoteProjectionProviderError> {
    let prefix = prefix.trim().trim_start_matches('/').trim_end_matches('/');
    if prefix.is_empty() {
        return Ok(String::new());
    }
    let mut parts = Vec::new();
    for segment in prefix.split('/') {
        if matches!(segment, "" | "." | "..") || segment.contains(':') {
            return Err(profile_error(
                "S3 custom endpoint profile allowed_prefix contains an unsafe segment",
            ));
        }
        parts.push(segment);
    }
    Ok(format!("{}/", parts.join("/")))
}

fn prefix_is_within_allowed_root(prefix: &str, allowed: &str) -> bool {
    allowed.is_empty() || prefix == allowed || prefix.starts_with(allowed)
}

fn validate_allowed_capabilities(
    capabilities: &[String],
) -> Result<(), RemoteProjectionProviderError> {
    if capabilities.is_empty() {
        return Err(profile_error(
            "S3 custom endpoint profile allowed_capabilities must not be empty",
        ));
    }
    for capability in capabilities {
        let capability = capability.trim();
        if !(capability.eq_ignore_ascii_case("push")
            || capability.eq_ignore_ascii_case("source-acquisition"))
        {
            return Err(profile_error(
                "S3 custom endpoint profile allowed_capabilities must contain only push or source-acquisition",
            ));
        }
    }
    Ok(())
}

fn validate_env_prefix(prefix: &str) -> Result<(), RemoteProjectionProviderError> {
    let prefix = prefix.trim();
    if prefix.is_empty() {
        return Err(profile_error(
            "S3 custom endpoint profile credential_ref.env_prefix is required",
        ));
    }
    let mut bytes = prefix.bytes();
    let Some(first) = bytes.next() else {
        return Err(profile_error(
            "S3 custom endpoint profile credential_ref.env_prefix is required",
        ));
    };
    if !(first.is_ascii_alphabetic() || first == b'_')
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(profile_error(
            "S3 custom endpoint profile credential_ref.env_prefix must be a valid environment variable prefix",
        ));
    }
    Ok(())
}

fn required_prefixed_env(
    prefix: &str,
    suffix: &str,
) -> Result<String, RemoteProjectionProviderError> {
    let name = format!("{prefix}_{suffix}");
    optional_env(&name).ok_or_else(|| {
        profile_error(format!(
            "S3 custom endpoint credential environment variable {name} is not configured"
        ))
    })
}

fn optional_prefixed_env(prefix: &str, suffix: &str) -> Option<String> {
    optional_env(&format!("{prefix}_{suffix}"))
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn default_addressing_style() -> String {
    ADDRESSING_STYLE_PATH.to_string()
}

fn profile_error(message: impl Into<String>) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(message.into())
}

#[cfg(test)]
mod tests;

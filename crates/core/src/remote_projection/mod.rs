//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 06_backup#remote-projection-transport-contract
//!
//! Admission and push contracts for Markdown projection remote transports.
//!
//! Locator validation is capability-neutral. Host transport adapters decide
//! whether an admitted locator is used for Projection push or immutable source
//! acquisition; neither capability gains Ledger or Source Control authority.

use thiserror::Error;

pub use crate::protocol::RemoteProjectionProvider;

mod provider;

pub use provider::{
    RemoteProjectionAuthorityEffects, RemoteProjectionFile, RemoteProjectionProviderAdapter,
    RemoteProjectionProviderError, RemoteProjectionPushOutcome, RemoteProjectionPushRequest,
};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RemoteProjectionError {
    #[error("remote projection locator is empty")]
    EmptyLocator,
    #[error("remote projection locator scheme does not match provider")]
    ProviderSchemeMismatch,
    #[error("remote projection locator must not contain credentials, query, or fragment data")]
    SecretMaterialForbidden,
    #[error(
        "remote projection locator is missing a provider host, namespace, or projection prefix"
    )]
    IncompleteLocator,
    #[error("remote projection locator contains an unsafe path segment")]
    UnsafeRemotePath,
}

/// Validates and normalizes a provider-bound Remote Projection locator.
///
/// This API deliberately carries no push/import direction. Capability and
/// credential admission remain host-runtime concerns after this structural
/// locator check succeeds.
pub fn validate_remote_projection_locator(
    provider: RemoteProjectionProvider,
    locator: &str,
) -> Result<String, RemoteProjectionError> {
    let locator = locator.trim();
    if locator.is_empty() {
        return Err(RemoteProjectionError::EmptyLocator);
    }
    if locator.contains('?') || locator.contains('#') {
        return Err(RemoteProjectionError::SecretMaterialForbidden);
    }
    if locator_contains_credentials(locator) {
        return Err(RemoteProjectionError::SecretMaterialForbidden);
    }
    if !locator_scheme_matches(provider, locator) {
        return Err(RemoteProjectionError::ProviderSchemeMismatch);
    }
    validate_locator_shape(provider, locator)?;
    if locator_has_unsafe_path_segment(locator) {
        return Err(RemoteProjectionError::UnsafeRemotePath);
    }
    Ok(locator.to_string())
}

fn validate_locator_shape(
    provider: RemoteProjectionProvider,
    locator: &str,
) -> Result<(), RemoteProjectionError> {
    let Some((_, after_scheme)) = locator.split_once("://") else {
        return Err(RemoteProjectionError::IncompleteLocator);
    };
    let (authority, path) = after_scheme.split_once('/').unwrap_or((after_scheme, ""));
    if authority.is_empty()
        || authority.starts_with(':')
        || authority.contains(['\\', ' ', '\t', '\r', '\n'])
    {
        return Err(RemoteProjectionError::IncompleteLocator);
    }

    let path = path.trim_end_matches('/');
    let segment_count = if path.is_empty() {
        0
    } else {
        path.split('/').count()
    };
    let required_segments = match provider {
        RemoteProjectionProvider::WebDav => 1,
        RemoteProjectionProvider::S3 if locator_starts_with_scheme(locator, "s3+https://") => 2,
        RemoteProjectionProvider::S3 => 1,
    };
    if segment_count < required_segments {
        return Err(RemoteProjectionError::IncompleteLocator);
    }
    if provider == RemoteProjectionProvider::S3
        && locator_starts_with_scheme(locator, "s3://")
        && authority.contains(':')
    {
        return Err(RemoteProjectionError::IncompleteLocator);
    }
    Ok(())
}

fn locator_scheme_matches(provider: RemoteProjectionProvider, locator: &str) -> bool {
    match provider {
        RemoteProjectionProvider::WebDav => locator_starts_with_scheme(locator, "webdav+https://"),
        RemoteProjectionProvider::S3 => {
            locator_starts_with_scheme(locator, "s3://")
                || locator_starts_with_scheme(locator, "s3+https://")
        }
    }
}

fn locator_starts_with_scheme(locator: &str, scheme: &str) -> bool {
    locator
        .get(..scheme.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme))
}

fn locator_contains_credentials(locator: &str) -> bool {
    let Some(after_scheme) = locator.split_once("://").map(|(_, rest)| rest) else {
        return false;
    };
    let authority = after_scheme.split('/').next().unwrap_or_default();
    authority.contains('@')
}

fn locator_has_unsafe_path_segment(locator: &str) -> bool {
    let Some(after_scheme) = locator.split_once("://").map(|(_, rest)| rest) else {
        return true;
    };
    let mut parts = after_scheme.split('/').skip(1).collect::<Vec<_>>();
    if parts.last() == Some(&"") {
        parts.pop();
    }
    parts.into_iter().any(|segment| {
        matches!(segment, "" | "." | "..")
            || segment.contains(['\\', '%'])
            || segment.chars().any(char::is_whitespace)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_returns_trimmed_locator_without_capability_semantics() {
        let locator = validate_remote_projection_locator(
            RemoteProjectionProvider::WebDav,
            "  webdav+https://dav.example.com/notebooks/main\n",
        )
        .expect("valid locator");

        assert_eq!(locator, "webdav+https://dav.example.com/notebooks/main");
    }

    #[test]
    fn locator_allows_one_optional_trailing_slash_but_not_empty_inner_segments() {
        for locator in [
            "webdav+https://dav.example.com/notebooks/main/",
            "s3://bucket/notebooks/main/",
        ] {
            let provider = if locator.starts_with("s3://") {
                RemoteProjectionProvider::S3
            } else {
                RemoteProjectionProvider::WebDav
            };
            validate_remote_projection_locator(provider, locator).expect("one trailing slash");
        }

        for locator in [
            "webdav+https://dav.example.com/notebooks//main",
            "s3://bucket/notebooks/main//",
        ] {
            let provider = if locator.starts_with("s3://") {
                RemoteProjectionProvider::S3
            } else {
                RemoteProjectionProvider::WebDav
            };
            assert!(
                validate_remote_projection_locator(provider, locator).is_err(),
                "{locator}"
            );
        }
    }

    #[test]
    fn transport_locator_scheme_matching_is_case_insensitive() {
        let s3 = validate_remote_projection_locator(
            RemoteProjectionProvider::S3,
            "S3://bucket/notebooks/main",
        )
        .expect("uppercase s3 scheme");
        assert_eq!(s3, "S3://bucket/notebooks/main");

        let s3_custom = validate_remote_projection_locator(
            RemoteProjectionProvider::S3,
            "S3+HTTPS://minio.example.com/bucket/notebooks/main",
        )
        .expect("uppercase s3 custom endpoint scheme");
        assert_eq!(
            s3_custom,
            "S3+HTTPS://minio.example.com/bucket/notebooks/main"
        );

        let webdav = validate_remote_projection_locator(
            RemoteProjectionProvider::WebDav,
            "WEBDAV+HTTPS://dav.example.com/notebooks/main",
        )
        .expect("uppercase webdav scheme");
        assert_eq!(webdav, "WEBDAV+HTTPS://dav.example.com/notebooks/main");
    }

    #[test]
    fn rejects_wrong_scheme_or_secret_material() {
        assert_eq!(
            validate_remote_projection_locator(RemoteProjectionProvider::S3, "  ")
                .expect_err("empty locator"),
            RemoteProjectionError::EmptyLocator
        );
        assert_eq!(
            validate_remote_projection_locator(
                RemoteProjectionProvider::WebDav,
                "s3://bucket/notebooks",
            )
            .expect_err("scheme"),
            RemoteProjectionError::ProviderSchemeMismatch
        );
        assert_eq!(
            validate_remote_projection_locator(
                RemoteProjectionProvider::S3,
                "s3://token@bucket/notebooks",
            )
            .expect_err("secret"),
            RemoteProjectionError::SecretMaterialForbidden
        );
        assert_eq!(
            validate_remote_projection_locator(
                RemoteProjectionProvider::S3,
                "s3://bucket/notebooks/../secrets",
            )
            .expect_err("unsafe path"),
            RemoteProjectionError::UnsafeRemotePath
        );
    }

    #[test]
    fn rejects_missing_provider_authority_namespace_or_prefix() {
        for (provider, locator) in [
            (
                RemoteProjectionProvider::WebDav,
                "webdav+https:///notebooks",
            ),
            (
                RemoteProjectionProvider::WebDav,
                "webdav+https://dav.example.com/",
            ),
            (RemoteProjectionProvider::S3, "s3:///notebooks"),
            (RemoteProjectionProvider::S3, "s3://bucket/"),
            (RemoteProjectionProvider::S3, "s3+https:///bucket/notebooks"),
            (
                RemoteProjectionProvider::S3,
                "s3+https://r2.example.com/bucket",
            ),
        ] {
            assert_eq!(
                validate_remote_projection_locator(provider, locator)
                    .expect_err("incomplete locator"),
                RemoteProjectionError::IncompleteLocator,
                "{locator}"
            );
        }
    }

    #[test]
    fn rejects_encoded_or_backslash_path_segments() {
        for locator in [
            "webdav+https://dav.example.com/notebooks/%2e%2e/secrets",
            "s3://bucket/notebooks\\secrets",
        ] {
            let provider = if locator.starts_with("s3://") {
                RemoteProjectionProvider::S3
            } else {
                RemoteProjectionProvider::WebDav
            };
            assert_eq!(
                validate_remote_projection_locator(provider, locator)
                    .expect_err("unsafe encoded path"),
                RemoteProjectionError::UnsafeRemotePath,
                "{locator}"
            );
        }
    }
}

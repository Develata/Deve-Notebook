use super::*;
use crate::backup::{BackupSecretRefScheme, parse_backup_credential_ref, parse_backup_key_ref};

fn credential_ref() -> BackupSecretRef {
    parse_backup_credential_ref("env:DEVE_BACKUP_TOKEN").unwrap()
}

fn key_ref() -> BackupSecretRef {
    parse_backup_key_ref("keyring:deve/default-backup-key").unwrap()
}

fn input(locator: BackupLocator) -> BackupProviderDispatchInput {
    BackupProviderDispatchInput {
        locator,
        credential_ref: credential_ref(),
        key_ref: key_ref(),
    }
}

#[test]
fn dispatches_webdav_adapter_without_resolving_secret_material() {
    let locator = BackupLocator::parse("webdav+https://dav.example.com/notebooks/deve").unwrap();

    let plan = dispatch_backup_provider_adapter(input(locator)).expect("adapter plan");

    assert_eq!(plan.provider, BackupProviderKind::WebDavHttps);
    assert_eq!(plan.endpoint.as_deref(), Some("https://dav.example.com"));
    assert_eq!(plan.namespace, "dav.example.com");
    assert_eq!(plan.repo_root_path, "notebooks/deve");
    assert_eq!(plan.credential_ref.kind, BackupSecretRefKind::Credential);
    assert_eq!(plan.key_ref.kind, BackupSecretRefKind::Key);
    assert!(plan.supports_remote_listing);
    assert!(plan.provider_metadata_is_diagnostic_only);
}

#[test]
fn dispatches_s3_adapter_without_endpoint() {
    let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();

    let plan = dispatch_backup_provider_adapter(input(locator)).expect("adapter plan");

    assert_eq!(plan.provider, BackupProviderKind::S3);
    assert_eq!(plan.endpoint, None);
    assert_eq!(plan.namespace, "bucket-name");
    assert_eq!(plan.repo_root_path, "deve");
}

#[test]
fn rejects_s3_endpoint_override() {
    let mut locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
    locator.endpoint = Some("https://s3.example.com".into());

    let err = dispatch_backup_provider_adapter(input(locator)).expect_err("s3 endpoint");

    assert_eq!(err, BackupProviderDispatchError::EndpointForbidden);
}

#[test]
fn rejects_missing_endpoint_for_https_backed_adapters() {
    let mut locator = BackupLocator::parse("s3+https://r2.example.com/bucket-name/deve").unwrap();
    locator.endpoint = None;

    let err = dispatch_backup_provider_adapter(input(locator)).expect_err("missing endpoint");

    assert_eq!(err, BackupProviderDispatchError::MissingEndpoint);
}

#[test]
fn rejects_non_https_endpoint_for_https_backed_adapters() {
    let mut locator =
        BackupLocator::parse("webdav+https://dav.example.com/notebooks/deve").unwrap();
    locator.endpoint = Some("http://dav.example.com".into());

    let err = dispatch_backup_provider_adapter(input(locator)).expect_err("non https endpoint");

    assert_eq!(err, BackupProviderDispatchError::NonHttpsEndpoint);
}

#[test]
fn rejects_secret_ref_kind_mismatch() {
    let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
    let mut input = input(locator);
    input.credential_ref = BackupSecretRef {
        kind: BackupSecretRefKind::Key,
        scheme: BackupSecretRefScheme::Env,
        name: "DEVE_BACKUP_KEY".into(),
    };

    let err = dispatch_backup_provider_adapter(input).expect_err("credential kind mismatch");

    assert_eq!(err, BackupProviderDispatchError::CredentialRefKindMismatch);
}

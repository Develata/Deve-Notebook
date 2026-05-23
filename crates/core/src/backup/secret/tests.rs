use super::{
    BackupSecretRefError, BackupSecretRefKind, BackupSecretRefScheme, parse_backup_credential_ref,
    parse_backup_key_ref,
};

#[test]
fn parses_credential_and_key_references_without_loading_secret_material() {
    let credential = parse_backup_credential_ref("env:DEVE_BACKUP_TOKEN").unwrap();
    assert_eq!(credential.kind, BackupSecretRefKind::Credential);
    assert_eq!(credential.scheme, BackupSecretRefScheme::Env);
    assert_eq!(credential.name, "DEVE_BACKUP_TOKEN");
    assert_eq!(credential.redacted(), "env:<redacted>");

    let key = parse_backup_key_ref("keyring:deve/default-backup-key").unwrap();
    assert_eq!(key.kind, BackupSecretRefKind::Key);
    assert_eq!(key.scheme, BackupSecretRefScheme::Keyring);
    assert_eq!(key.name, "deve/default-backup-key");
    assert_eq!(key.redacted(), "keyring:<redacted>");
}

#[test]
fn accepts_config_refs_for_runtime_config_indirection() {
    let credential = parse_backup_credential_ref("config:backup.credential_ref").unwrap();

    assert_eq!(credential.scheme, BackupSecretRefScheme::Config);
    assert_eq!(credential.name, "backup.credential_ref");
}

#[test]
fn rejects_raw_secret_material_and_url_like_values() {
    for input in [
        "https://user:pass@example.com",
        "env:DEVE_TOKEN?debug=true",
        "token=abc123",
        "password=hunter2",
        "secret=abc123",
        "access_key=abc123",
        "secret_key=abc123",
        "-----BEGIN PRIVATE KEY-----",
    ] {
        assert!(matches!(
            parse_backup_credential_ref(input),
            Err(BackupSecretRefError::SecretMaterialForbidden)
        ));
    }
}

#[test]
fn rejects_unknown_scheme_empty_ref_and_unsafe_names() {
    assert!(matches!(
        parse_backup_credential_ref(""),
        Err(BackupSecretRefError::EmptyReference)
    ));
    assert!(matches!(
        parse_backup_credential_ref("plain-name"),
        Err(BackupSecretRefError::UnsupportedReferenceScheme)
    ));
    assert!(matches!(
        parse_backup_credential_ref("file:/tmp/secret"),
        Err(BackupSecretRefError::UnsupportedReferenceScheme)
    ));
    assert!(matches!(
        parse_backup_credential_ref("env:deve_backup_token"),
        Err(BackupSecretRefError::UnsafeReferenceName)
    ));
    assert!(matches!(
        parse_backup_credential_ref(" env:DEVE_BACKUP_TOKEN"),
        Err(BackupSecretRefError::UnsafeReferenceName)
    ));
    assert!(matches!(
        parse_backup_key_ref("keyring:deve/../backup-key"),
        Err(BackupSecretRefError::UnsafeReferenceName)
    ));
    assert!(matches!(
        parse_backup_key_ref("config:backup..key_ref"),
        Err(BackupSecretRefError::UnsafeReferenceName)
    ));
}

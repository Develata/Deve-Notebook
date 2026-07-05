//! plan_ref:
//!   - 06_backup#backup-secret-ref-contract

use anyhow::{Context, bail};
use deve_core::backup::{
    BackupArtifactKey, BackupSecretRef, BackupSecretRefKind, BackupSecretRefScheme,
};
use serde::Deserialize;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct S3BackupCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
    pub region: String,
}

#[derive(Deserialize)]
struct S3CredentialEnvelope {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
    region: Option<String>,
}

pub(super) fn webdav_authorization_header(ref_: &BackupSecretRef) -> anyhow::Result<String> {
    let value = read_env_credential(ref_)?;
    if value.contains('\r') || value.contains('\n') {
        bail!("backup WebDAV authorization header contains unsafe characters");
    }
    Ok(value)
}

pub(super) fn s3_credentials(ref_: &BackupSecretRef) -> anyhow::Result<S3BackupCredentials> {
    let value = read_env_credential(ref_)?;
    s3_credentials_from_json(&value)
}

pub(super) fn backup_artifact_key(ref_: &BackupSecretRef) -> anyhow::Result<BackupArtifactKey> {
    let value = read_env_key(ref_)?;
    let bytes = decode_hex_key(&value)?;
    BackupArtifactKey::from_bytes(&bytes)
        .context("backup key env ref did not resolve to a 32-byte key")
}

fn read_env_credential(ref_: &BackupSecretRef) -> anyhow::Result<String> {
    if ref_.kind != BackupSecretRefKind::Credential {
        bail!("backup provider upload credential ref has the wrong kind");
    }
    if ref_.scheme != BackupSecretRefScheme::Env {
        bail!("backup provider upload currently resolves only env: credential refs");
    }
    let value = std::env::var(&ref_.name)
        .with_context(|| format!("backup credential env ref {} is not configured", ref_.name))?;
    let value = value.trim().to_string();
    if value.is_empty() {
        bail!("backup credential env ref {} is empty", ref_.name);
    }
    Ok(value)
}

fn read_env_key(ref_: &BackupSecretRef) -> anyhow::Result<String> {
    if ref_.kind != BackupSecretRefKind::Key {
        bail!("backup key ref has the wrong kind");
    }
    if ref_.scheme != BackupSecretRefScheme::Env {
        bail!("backup key resolver currently resolves only env: key refs");
    }
    let value = std::env::var(&ref_.name)
        .with_context(|| format!("backup key env ref {} is not configured", ref_.name))?;
    let value = value.trim().to_string();
    if value.is_empty() {
        bail!("backup key env ref {} is empty", ref_.name);
    }
    Ok(value)
}

fn s3_credentials_from_json(input: &str) -> anyhow::Result<S3BackupCredentials> {
    let envelope: S3CredentialEnvelope = serde_json::from_str(input)
        .context("backup S3 credential env value must be a JSON object")?;
    let access_key_id = required_secret_field("access_key_id", envelope.access_key_id)?;
    let secret_access_key = required_secret_field("secret_access_key", envelope.secret_access_key)?;
    let region = envelope
        .region
        .and_then(non_empty)
        .or_else(|| optional_env("AWS_REGION"))
        .or_else(|| optional_env("AWS_DEFAULT_REGION"))
        .context("backup S3 region is not configured")?;
    Ok(S3BackupCredentials {
        access_key_id,
        secret_access_key,
        session_token: envelope.session_token.and_then(non_empty),
        region,
    })
}

fn required_secret_field(name: &str, value: String) -> anyhow::Result<String> {
    non_empty(value).with_context(|| format!("backup S3 credential field {name} is empty"))
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(non_empty)
}

fn decode_hex_key(input: &str) -> anyhow::Result<[u8; 32]> {
    if input.len() != 64 || !input.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("backup key env value must be 64 hex characters");
    }
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&input[start..start + 2], 16)
            .map_err(|_| anyhow::anyhow!("backup key env value must be 64 hex characters"))?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::backup::{parse_backup_credential_ref, parse_backup_key_ref};

    #[test]
    fn parses_s3_credential_json_without_logging_secret_material() {
        let credentials = s3_credentials_from_json(
            r#"{"access_key_id":"AKID","secret_access_key":"SECRET","session_token":"TOKEN","region":"us-east-1"}"#,
        )
        .unwrap();

        assert_eq!(credentials.access_key_id, "AKID");
        assert_eq!(credentials.secret_access_key, "SECRET");
        assert_eq!(credentials.session_token.as_deref(), Some("TOKEN"));
        assert_eq!(credentials.region, "us-east-1");
    }

    #[test]
    fn rejects_incomplete_s3_credential_json() {
        let err = match s3_credentials_from_json(
            r#"{"access_key_id":"","secret_access_key":"SECRET"}"#,
        ) {
            Ok(_) => panic!("empty access key must fail"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("access_key_id"));
        assert!(!err.to_string().contains("SECRET"));
    }

    #[test]
    fn rejects_non_env_credential_refs_for_provider_upload() {
        let keyring_ref = parse_backup_credential_ref("keyring:deve/backup-token").unwrap();
        let config_ref = parse_backup_credential_ref("config:backup.token").unwrap();

        let keyring_err = match webdav_authorization_header(&keyring_ref) {
            Ok(_) => panic!("keyring resolver must fail closed"),
            Err(err) => err,
        };
        let config_err = match s3_credentials(&config_ref) {
            Ok(_) => panic!("config resolver must fail closed"),
            Err(err) => err,
        };

        assert!(keyring_err.to_string().contains("only env"));
        assert!(config_err.to_string().contains("only env"));
    }

    #[test]
    fn parses_hex_backup_artifact_key_without_logging_material() {
        let bytes =
            decode_hex_key("0707070707070707070707070707070707070707070707070707070707070707")
                .expect("hex key");

        assert_eq!(bytes, [7; 32]);
    }

    #[test]
    fn rejects_non_env_key_refs_for_runtime_key_resolution() {
        let keyring_ref = parse_backup_key_ref("keyring:deve/backup-key").unwrap();
        let config_ref = parse_backup_key_ref("config:backup.key_ref").unwrap();

        let keyring_err = match backup_artifact_key(&keyring_ref) {
            Ok(_) => panic!("keyring resolver must fail closed"),
            Err(err) => err,
        };
        let config_err = match backup_artifact_key(&config_ref) {
            Ok(_) => panic!("config resolver must fail closed"),
            Err(err) => err,
        };

        assert!(keyring_err.to_string().contains("only env"));
        assert!(config_err.to_string().contains("only env"));
    }
}

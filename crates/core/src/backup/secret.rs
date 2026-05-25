//! plan_ref:
//!   - 06_backup#backup-secret-ref-contract
//!
//! Backup credential/key reference validation.
//!
//! This module validates that backup credentials and encryption keys enter the
//! runtime as references only. It does not read environment variables, open a
//! keyring, load config, decrypt material, or log raw secret values.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupSecretRefKind {
    Credential,
    Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupSecretRefScheme {
    Env,
    Keyring,
    Config,
}

impl BackupSecretRefScheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Keyring => "keyring",
            Self::Config => "config",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSecretRef {
    pub kind: BackupSecretRefKind,
    pub scheme: BackupSecretRefScheme,
    pub name: String,
}

impl BackupSecretRef {
    pub fn redacted(&self) -> String {
        format!("{}:<redacted>", self.scheme.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupSecretRefError {
    #[error("backup secret reference is empty")]
    EmptyReference,
    #[error("backup secret must be supplied as env:, keyring:, or config: reference")]
    UnsupportedReferenceScheme,
    #[error("backup secret reference contains unsafe characters")]
    UnsafeReferenceName,
    #[error("backup secret reference appears to contain raw secret material")]
    SecretMaterialForbidden,
}

pub fn parse_backup_credential_ref(input: &str) -> Result<BackupSecretRef, BackupSecretRefError> {
    parse_backup_secret_ref(input, BackupSecretRefKind::Credential)
}

pub fn parse_backup_key_ref(input: &str) -> Result<BackupSecretRef, BackupSecretRefError> {
    parse_backup_secret_ref(input, BackupSecretRefKind::Key)
}

fn parse_backup_secret_ref(
    input: &str,
    kind: BackupSecretRefKind,
) -> Result<BackupSecretRef, BackupSecretRefError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(BackupSecretRefError::EmptyReference);
    }
    if trimmed != input {
        return Err(BackupSecretRefError::UnsafeReferenceName);
    }
    let input = trimmed;
    reject_raw_secret_material(input)?;

    let (scheme, name) = input
        .split_once(':')
        .ok_or(BackupSecretRefError::UnsupportedReferenceScheme)?;
    let scheme = parse_scheme(scheme)?;
    validate_reference_name(scheme, name)?;

    Ok(BackupSecretRef {
        kind,
        scheme,
        name: name.to_string(),
    })
}

fn parse_scheme(input: &str) -> Result<BackupSecretRefScheme, BackupSecretRefError> {
    match input {
        "env" => Ok(BackupSecretRefScheme::Env),
        "keyring" => Ok(BackupSecretRefScheme::Keyring),
        "config" => Ok(BackupSecretRefScheme::Config),
        _ => Err(BackupSecretRefError::UnsupportedReferenceScheme),
    }
}

fn validate_reference_name(
    scheme: BackupSecretRefScheme,
    name: &str,
) -> Result<(), BackupSecretRefError> {
    if name.is_empty() || name.trim() != name {
        return Err(BackupSecretRefError::UnsafeReferenceName);
    }
    if name
        .chars()
        .any(|ch| ch.is_ascii_control() || matches!(ch, '\0' | '?' | '#' | '\\'))
    {
        return Err(BackupSecretRefError::UnsafeReferenceName);
    }
    match scheme {
        BackupSecretRefScheme::Env => validate_env_ref(name),
        BackupSecretRefScheme::Keyring => validate_path_like_ref(name),
        BackupSecretRefScheme::Config => validate_config_ref(name),
    }
}

fn validate_env_ref(name: &str) -> Result<(), BackupSecretRefError> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(BackupSecretRefError::UnsafeReferenceName);
    };
    if !(first.is_ascii_uppercase() || first == '_') {
        return Err(BackupSecretRefError::UnsafeReferenceName);
    }
    if !chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_') {
        return Err(BackupSecretRefError::UnsafeReferenceName);
    }
    Ok(())
}

fn validate_path_like_ref(name: &str) -> Result<(), BackupSecretRefError> {
    for segment in name.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(BackupSecretRefError::UnsafeReferenceName);
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(BackupSecretRefError::UnsafeReferenceName);
        }
    }
    Ok(())
}

fn validate_config_ref(name: &str) -> Result<(), BackupSecretRefError> {
    for segment in name.split('.') {
        if segment.is_empty() {
            return Err(BackupSecretRefError::UnsafeReferenceName);
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        {
            return Err(BackupSecretRefError::UnsafeReferenceName);
        }
    }
    Ok(())
}

fn reject_raw_secret_material(input: &str) -> Result<(), BackupSecretRefError> {
    let lower = input.to_ascii_lowercase();
    if input.contains("://")
        || input.contains('?')
        || input.contains('#')
        || input.contains('\0')
        || lower.contains("token=")
        || lower.contains("password=")
        || lower.contains("secret=")
        || lower.contains("access_key")
        || lower.contains("secret_key")
        || lower.contains("-----begin ")
    {
        return Err(BackupSecretRefError::SecretMaterialForbidden);
    }
    Ok(())
}

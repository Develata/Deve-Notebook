//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

use super::super::BACKUP_PACK_CONTENT_TYPE;
use crate::commands::backup::provider_io::credentials::S3BackupCredentials;
use anyhow::Context;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::Url;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct S3SignedBackupPutRequest {
    pub(super) url: Url,
    pub(super) body: Vec<u8>,
    pub(super) headers: Vec<(String, String)>,
}

pub(super) fn signed_put_request(
    url: Url,
    body: Vec<u8>,
    credentials: &S3BackupCredentials,
    now: DateTime<Utc>,
) -> anyhow::Result<S3SignedBackupPutRequest> {
    let payload_hash = sha256_hex(&body);
    let headers = signed_headers(
        "PUT",
        &url,
        payload_hash.clone(),
        BTreeMap::from([(
            "content-type".to_string(),
            BACKUP_PACK_CONTENT_TYPE.to_string(),
        )]),
        credentials,
        now,
    )?;

    Ok(S3SignedBackupPutRequest { url, body, headers })
}

fn signed_headers(
    method: &str,
    url: &Url,
    payload_hash: String,
    mut signed_headers: BTreeMap<String, String>,
    credentials: &S3BackupCredentials,
    now: DateTime<Utc>,
) -> anyhow::Result<Vec<(String, String)>> {
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let short_date = now.format("%Y%m%d").to_string();
    let host = canonical_host(url)?;
    signed_headers.insert("host".to_string(), host);
    signed_headers.insert("x-amz-content-sha256".to_string(), payload_hash.clone());
    signed_headers.insert("x-amz-date".to_string(), amz_date.clone());
    if let Some(token) = &credentials.session_token {
        signed_headers.insert("x-amz-security-token".to_string(), token.trim().to_string());
    }

    let canonical_headers = signed_headers
        .iter()
        .map(|(name, value)| format!("{name}:{}\n", normalize_header_value(value)))
        .collect::<String>();
    let signed_header_names = signed_headers
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(";");
    let canonical_request = format!(
        "{method}\n{}\n{}\n{}\n{}\n{}",
        canonical_uri(url),
        canonical_query(url),
        canonical_headers,
        signed_header_names,
        payload_hash
    );
    let credential_scope = format!("{short_date}/{}/s3/aws4_request", credentials.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let signature = sign_hex(
        &signing_key(
            &credentials.secret_access_key,
            &short_date,
            &credentials.region,
        ),
        string_to_sign.as_bytes(),
    );
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        credentials.access_key_id, credential_scope, signed_header_names, signature
    );

    let mut headers = signed_headers.into_iter().collect::<Vec<_>>();
    headers.push(("authorization".into(), authorization));
    Ok(headers)
}

fn signing_key(secret: &str, short_date: &str, region: &str) -> Vec<u8> {
    let date = hmac_sha256(format!("AWS4{secret}").as_bytes(), short_date.as_bytes());
    let region = hmac_sha256(&date, region.as_bytes());
    let service = hmac_sha256(&region, b"s3");
    hmac_sha256(&service, b"aws4_request")
}

fn canonical_host(url: &Url) -> anyhow::Result<String> {
    let host = url
        .host_str()
        .context("Backup S3 request URL has no host")?;
    Ok(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    })
}

fn canonical_uri(url: &Url) -> &str {
    if url.path().is_empty() {
        "/"
    } else {
        url.path()
    }
}

fn canonical_query(url: &Url) -> &str {
    url.query().unwrap_or("")
}

fn normalize_header_value(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sha256_hex(input: impl AsRef<[u8]>) -> String {
    hex_lower(&Sha256::digest(input.as_ref()))
}

fn sign_hex(key: &[u8], input: &[u8]) -> String {
    hex_lower(&hmac_sha256(key, input))
}

fn hmac_sha256(key: &[u8], input: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(input);
    mac.finalize().into_bytes().to_vec()
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!
//! Bearer-only, loopback-only admission for typed Remote Import CLI requests.

use crate::local_cli_proxy_contract::{LocalCliRemoteImportRequest, LocalCliRepoRemovalRequest};
use axum::http::{HeaderMap, StatusCode, header};
use deve_core::models::RepoId;
use deve_core::protocol::auth::AuthErrorCode;
use deve_core::security::AuthConfig;
use deve_core::security::auth::jwt;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub(crate) const LOCAL_CLI_PROXY_MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;
const MAX_LIVE_IDENTITIES: usize = 1024;
const IDENTITY_TTL: Duration = Duration::from_secs(5 * 60);

pub(crate) struct LocalCliProxyGateway {
    identities: Mutex<HashMap<(String, Uuid), IdentityEntry>>,
}

impl Default for LocalCliProxyGateway {
    fn default() -> Self {
        Self {
            identities: Mutex::new(HashMap::new()),
        }
    }
}

struct IdentityEntry {
    digest: [u8; 32],
    expires_at: Instant,
}

#[derive(Debug)]
pub(crate) struct LocalCliProxyAuthority {
    request_id: Uuid,
    repo_id: RepoId,
    operation: &'static str,
    principal_digest: String,
}

impl LocalCliProxyAuthority {
    pub(crate) fn request_id(&self) -> Uuid {
        self.request_id
    }

    pub(crate) fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub(crate) fn operation(&self) -> &'static str {
        self.operation
    }

    pub(crate) fn principal_digest(&self) -> &str {
        &self.principal_digest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalCliProxyRejection {
    pub(crate) status: StatusCode,
    pub(crate) code: AuthErrorCode,
}

impl LocalCliProxyGateway {
    pub(crate) fn admit(
        &self,
        peer: SocketAddr,
        headers: &HeaderMap,
        config: &AuthConfig,
        body: &[u8],
    ) -> Result<(LocalCliProxyAuthority, LocalCliRemoteImportRequest), LocalCliProxyRejection> {
        let authenticated = authenticate(peer, headers, config, body)?;
        let request =
            serde_json::from_slice::<LocalCliRemoteImportRequest>(body).map_err(|_| forbidden())?;
        let identity = request.exact_identity();
        let repo_id = identity.repo_id;
        let branch = identity.branch;
        let scope_nonce = identity.scope_nonce;
        let session_id = identity.session_id;
        let revision = identity.revision;
        if repo_id.is_nil() || branch.is_some() || scope_nonce.get() == 0 {
            return Err(forbidden());
        }
        let identity_digest = exact_identity_digest(
            "remote-import",
            request.operation_name(),
            repo_id,
            scope_nonce.get(),
            session_id,
            revision,
            body,
        );
        self.admit_identity(&authenticated.sid, request.request_id(), identity_digest)?;
        Ok((
            LocalCliProxyAuthority {
                request_id: request.request_id(),
                repo_id,
                operation: request.operation_name(),
                principal_digest: authenticated.principal_digest,
            },
            request,
        ))
    }

    pub(crate) fn admit_repo_removal(
        &self,
        peer: SocketAddr,
        headers: &HeaderMap,
        config: &AuthConfig,
        body: &[u8],
    ) -> Result<(LocalCliProxyAuthority, LocalCliRepoRemovalRequest), LocalCliProxyRejection> {
        let authenticated = authenticate(peer, headers, config, body)?;
        let request =
            serde_json::from_slice::<LocalCliRepoRemovalRequest>(body).map_err(|_| forbidden())?;
        let repo_id = request.repo_id();
        let request_id = request.request_id();
        let (scope_nonce, switch_nonce, preparation_id) = request.scope_identity();
        if repo_id.is_nil() || request_id.is_nil() {
            return Err(forbidden());
        }
        match &request {
            LocalCliRepoRemovalRequest::Prepare { .. } if scope_nonce == 0 => {
                return Err(forbidden());
            }
            LocalCliRepoRemovalRequest::Execute { .. }
                if preparation_id.is_none_or(|value| value.is_nil())
                    || scope_nonce == 0
                    || switch_nonce.is_none_or(|value| value <= scope_nonce) =>
            {
                return Err(forbidden());
            }
            LocalCliRepoRemovalRequest::Status {
                execute_request_id, ..
            } if execute_request_id.is_nil() => return Err(forbidden()),
            _ => {}
        }
        let digest = exact_identity_digest(
            "repo-removal",
            request.operation_name(),
            repo_id,
            scope_nonce,
            preparation_id,
            switch_nonce,
            body,
        );
        self.admit_identity(&authenticated.sid, request_id, digest)?;
        Ok((
            LocalCliProxyAuthority {
                request_id,
                repo_id,
                operation: request.operation_name(),
                principal_digest: authenticated.principal_digest,
            },
            request,
        ))
    }

    fn admit_identity(
        &self,
        sid: &str,
        request_id: Uuid,
        digest: [u8; 32],
    ) -> Result<(), LocalCliProxyRejection> {
        let now = Instant::now();
        let mut identities = self.identities.lock().map_err(|_| internal())?;
        identities.retain(|_, entry| entry.expires_at > now);
        let key = (sid.to_string(), request_id);
        if let Some(existing) = identities.get(&key) {
            return if existing.digest == digest {
                Ok(())
            } else {
                Err(forbidden())
            };
        }
        if identities.len() >= MAX_LIVE_IDENTITIES {
            return Err(internal());
        }
        identities.insert(
            key,
            IdentityEntry {
                digest,
                expires_at: now + IDENTITY_TTL,
            },
        );
        Ok(())
    }
}

fn exact_identity_digest(
    family: &str,
    operation: &str,
    repo_id: RepoId,
    scope_nonce: u64,
    session_id: Option<Uuid>,
    revision: Option<u64>,
    body: &[u8],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"deve-local-cli-proxy-identity-v2\0");
    hash_field(&mut hasher, family.as_bytes());
    hash_field(&mut hasher, operation.as_bytes());
    hash_field(&mut hasher, repo_id.as_bytes());
    hash_field(&mut hasher, &scope_nonce.to_le_bytes());
    match session_id {
        Some(session_id) => hash_field(&mut hasher, session_id.as_bytes()),
        None => hash_field(&mut hasher, &[]),
    }
    match revision {
        Some(revision) => hash_field(&mut hasher, &revision.to_le_bytes()),
        None => hash_field(&mut hasher, &[]),
    }
    hash_field(&mut hasher, &Sha256::digest(body));
    hasher.finalize().into()
}

fn authenticate(
    peer: SocketAddr,
    headers: &HeaderMap,
    config: &AuthConfig,
    body: &[u8],
) -> Result<AuthenticatedCli, LocalCliProxyRejection> {
    if !peer.ip().is_loopback()
        || headers.contains_key(header::COOKIE)
        || headers.contains_key(header::ORIGIN)
        || headers.contains_key("x-deve-source-control-delegation")
        || body.len() > LOCAL_CLI_PROXY_MAX_REQUEST_BODY_BYTES
    {
        return Err(forbidden());
    }
    let mut authorization_values = headers.get_all(header::AUTHORIZATION).iter();
    let authorization = authorization_values
        .next()
        .and_then(|value| value.to_str().ok())
        .ok_or_else(missing_token)?;
    if authorization_values.next().is_some() {
        return Err(missing_token());
    }
    let token = authorization
        .strip_prefix("Bearer ")
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(missing_token)?;
    let claims = jwt::validate_token(&config.secret, token, config.token_version)
        .map_err(|_| expired_token())?;
    if claims.sub != config.username {
        return Err(expired_token());
    }
    let sid = claims
        .sid
        .filter(|sid| !sid.trim().is_empty())
        .ok_or_else(expired_token)?;
    Ok(AuthenticatedCli {
        sid,
        // The removal confirmation is intentionally usable by a later CLI
        // process authenticated as the same operator.  Browser/session ids
        // remain replay-cache identities, not durable removal principals.
        principal_digest: principal_binding_digest(&claims.sub, config.token_version),
    })
}

struct AuthenticatedCli {
    sid: String,
    principal_digest: String,
}

fn principal_binding_digest(subject: &str, token_version: u32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"deve-local-cli-principal-v1\0");
    hash_field(&mut hasher, subject.as_bytes());
    hash_field(&mut hasher, &token_version.to_le_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hash_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value);
}

fn missing_token() -> LocalCliProxyRejection {
    LocalCliProxyRejection {
        status: StatusCode::UNAUTHORIZED,
        code: AuthErrorCode::TokenMissing,
    }
}

fn expired_token() -> LocalCliProxyRejection {
    LocalCliProxyRejection {
        status: StatusCode::UNAUTHORIZED,
        code: AuthErrorCode::TokenExpired,
    }
}

fn forbidden() -> LocalCliProxyRejection {
    LocalCliProxyRejection {
        status: StatusCode::FORBIDDEN,
        code: AuthErrorCode::CsrfMismatch,
    }
}

fn internal() -> LocalCliProxyRejection {
    LocalCliProxyRejection {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: AuthErrorCode::InternalError,
    }
}

#[cfg(test)]
mod tests;

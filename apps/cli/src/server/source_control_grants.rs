//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 08_auth#jwt-cookie-contract
//!
//! Session-bound Source Control HTTP write grants.

use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode};
use deve_core::security::hashing::sha256_hex;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const DEFAULT_GRANT_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Debug, Eq)]
pub(crate) struct AuthSessionId(String);

impl AuthSessionId {
    pub(crate) fn from_cookie_token(token: &str) -> Self {
        Self(format!(
            "jwt:{}",
            sha256_hex(format!("deve-auth-session:{token}").as_bytes())
        ))
    }

    pub(crate) fn from_dev_session_cookie(
        username: &str,
        token_version: u32,
        dev_session: &str,
    ) -> Self {
        Self(format!(
            "localhost-dev:{}",
            sha256_hex(
                format!("deve-localhost-session:{username}:{token_version}:{dev_session}")
                    .as_bytes()
            )
        ))
    }

    #[cfg(test)]
    pub(crate) fn for_test(seed: &str) -> Self {
        Self(format!("test:{}", sha256_hex(seed.as_bytes())))
    }
}

impl PartialEq for AuthSessionId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Hash for AuthSessionId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceControlGrantBranch {
    Local,
    Remote(PeerId),
}

impl SourceControlGrantBranch {
    pub(crate) fn from_active_branch(branch: Option<&PeerId>) -> Self {
        branch.cloned().map(Self::Remote).unwrap_or(Self::Local)
    }
}

#[derive(Clone, Debug)]
struct SourceControlWriteGrant {
    auth_session_id: AuthSessionId,
    repo_id: RepoId,
    branch: SourceControlGrantBranch,
    writer_peer_id: PeerId,
    scope_nonce: u64,
    expires_at: Instant,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct GrantKey {
    auth_session_id: AuthSessionId,
    repo_id: RepoId,
}

#[derive(Debug)]
pub(crate) struct SourceControlWriteGrants {
    grants: Mutex<HashMap<GrantKey, SourceControlWriteGrant>>,
    ttl: Duration,
}

impl SourceControlWriteGrants {
    pub(crate) fn new() -> Self {
        Self::with_ttl(DEFAULT_GRANT_TTL)
    }

    pub(crate) fn with_ttl(ttl: Duration) -> Self {
        Self {
            grants: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    pub(crate) fn grant(
        &self,
        auth_session_id: AuthSessionId,
        repo_id: RepoId,
        branch: SourceControlGrantBranch,
        writer_peer_id: PeerId,
        scope_nonce: u64,
    ) -> Result<(), ServerError> {
        let Ok(mut grants) = self.grants.lock() else {
            return Err(stale_grant("source control write grant unavailable"));
        };
        Self::retain_live(&mut grants);
        grants.retain(|key, _| key.auth_session_id != auth_session_id);
        let key = GrantKey {
            auth_session_id: auth_session_id.clone(),
            repo_id,
        };
        grants.insert(
            key,
            SourceControlWriteGrant {
                auth_session_id,
                repo_id,
                branch,
                writer_peer_id,
                scope_nonce,
                expires_at: Instant::now() + self.ttl,
            },
        );
        Ok(())
    }

    pub(crate) fn authorize_browser_local(
        &self,
        auth_session_id: &AuthSessionId,
        repo_id: RepoId,
        scope_nonce: u64,
    ) -> Result<PeerId, ServerError> {
        self.authorize_for_branch(
            auth_session_id,
            repo_id,
            SourceControlGrantBranch::Local,
            scope_nonce,
        )
    }

    /// Refresh the HTTP mutation lease after the caller has proved an exact,
    /// live browser writer binding on the current WebSocket session.
    pub(crate) fn refresh_browser_local_from_live_writer(
        &self,
        auth_session_id: AuthSessionId,
        repo_id: RepoId,
        writer_peer_id: PeerId,
        scope_nonce: u64,
    ) -> Result<(), ServerError> {
        self.grant(
            auth_session_id,
            repo_id,
            SourceControlGrantBranch::Local,
            writer_peer_id,
            scope_nonce,
        )
    }

    fn authorize_for_branch(
        &self,
        auth_session_id: &AuthSessionId,
        repo_id: RepoId,
        branch: SourceControlGrantBranch,
        scope_nonce: u64,
    ) -> Result<PeerId, ServerError> {
        let Ok(mut grants) = self.grants.lock() else {
            return Err(stale_grant("source control write grant unavailable"));
        };
        Self::retain_live(&mut grants);
        let key = GrantKey {
            auth_session_id: auth_session_id.clone(),
            repo_id,
        };
        let Some(grant) = grants.get(&key) else {
            return Err(stale_grant("source control write grant missing"));
        };
        if grant.auth_session_id != *auth_session_id
            || grant.repo_id != repo_id
            || grant.branch != branch
            || grant.scope_nonce != scope_nonce
        {
            return Err(stale_grant("source control write grant mismatch"));
        }
        if Instant::now() >= grant.expires_at {
            grants.remove(&key);
            return Err(stale_grant("source control write grant expired"));
        }
        Ok(grant.writer_peer_id.clone())
    }

    pub(crate) fn revoke_session(&self, auth_session_id: &AuthSessionId) {
        let Ok(mut grants) = self.grants.lock() else {
            return;
        };
        grants.retain(|key, _| &key.auth_session_id != auth_session_id);
    }

    fn retain_live(grants: &mut HashMap<GrantKey, SourceControlWriteGrant>) {
        let now = Instant::now();
        grants.retain(|_, grant| now < grant.expires_at);
    }
}

fn stale_grant(detail: impl Into<String>) -> ServerError {
    ServerError::with_detail(ServerErrorCode::ScStaleScope, detail)
}

#[cfg(test)]
mod tests;

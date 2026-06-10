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

    pub(crate) fn anonymous_localhost(username: &str, token_version: u32) -> Self {
        Self(format!(
            "localhost-dev:{}",
            sha256_hex(format!("deve-localhost-session:{username}:{token_version}").as_bytes())
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

#[derive(Clone, Debug)]
struct SourceControlWriteGrant {
    auth_session_id: AuthSessionId,
    repo_id: RepoId,
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
        writer_peer_id: PeerId,
        scope_nonce: u64,
    ) {
        let Ok(mut grants) = self.grants.lock() else {
            return;
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
                writer_peer_id,
                scope_nonce,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }

    pub(crate) fn authorize(
        &self,
        auth_session_id: &AuthSessionId,
        repo_id: RepoId,
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
        grants.retain(|_, grant| now <= grant.expires_at);
    }
}

fn stale_grant(detail: impl Into<String>) -> ServerError {
    ServerError::with_detail(ServerErrorCode::ScStaleScope, detail)
}

#[cfg(test)]
mod tests {
    use super::{AuthSessionId, SourceControlWriteGrants};
    use deve_core::models::PeerId;
    use deve_core::protocol::ServerErrorCode;
    use std::time::Duration;

    #[test]
    fn grant_authorizes_matching_session_repo_and_scope() {
        let grants = SourceControlWriteGrants::new();
        let auth = AuthSessionId::for_test("session");
        let repo_id = uuid::Uuid::new_v4();
        let writer = PeerId::new("writer");
        grants.grant(auth.clone(), repo_id, writer.clone(), 7);

        assert_eq!(grants.authorize(&auth, repo_id, 7).unwrap(), writer);
    }

    #[test]
    fn grant_rejects_missing_or_stale_scope() {
        let grants = SourceControlWriteGrants::new();
        let auth = AuthSessionId::for_test("session");
        let repo_id = uuid::Uuid::new_v4();
        grants.grant(auth.clone(), repo_id, PeerId::new("writer"), 7);

        let err = grants.authorize(&auth, repo_id, 8).unwrap_err();
        assert_eq!(err.code, ServerErrorCode::ScStaleScope);

        let other = AuthSessionId::for_test("other");
        let err = grants.authorize(&other, repo_id, 7).unwrap_err();
        assert_eq!(err.code, ServerErrorCode::ScStaleScope);
    }

    #[test]
    fn grant_replaces_previous_session_writer() {
        let grants = SourceControlWriteGrants::new();
        let auth = AuthSessionId::for_test("session");
        let first_repo = uuid::Uuid::new_v4();
        let second_repo = uuid::Uuid::new_v4();
        grants.grant(auth.clone(), first_repo, PeerId::new("first"), 1);
        grants.grant(auth.clone(), second_repo, PeerId::new("second"), 2);

        assert!(grants.authorize(&auth, first_repo, 1).is_err());
        assert_eq!(
            grants.authorize(&auth, second_repo, 2).unwrap(),
            PeerId::new("second")
        );
    }

    #[test]
    fn http_source_control_write_grant_revoked_on_ws_disconnect() {
        let grants = SourceControlWriteGrants::with_ttl(Duration::from_millis(0));
        let auth = AuthSessionId::for_test("session");
        let repo_id = uuid::Uuid::new_v4();
        grants.grant(auth.clone(), repo_id, PeerId::new("writer"), 7);
        assert!(grants.authorize(&auth, repo_id, 7).is_err());

        let grants = SourceControlWriteGrants::new();
        grants.grant(auth.clone(), repo_id, PeerId::new("writer"), 7);
        grants.revoke_session(&auth);
        assert!(grants.authorize(&auth, repo_id, 7).is_err());
    }
}

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "auth-baseline-check: $*" >&2
  exit 1
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file"; then
    fail "unexpected '$pattern' in $file"
  fi
}

# AUTH-013: host identity key permissions are owner-only and fail closed.
check_contains docs/acceptance-cases/08_auth.md "case_id: AUTH-013"
check_contains docs/acceptance-cases/08_auth.md "cargo test -p deve_cli identity_key_permissions_are_corrected_to_owner_only -- --nocapture"
check_contains docs/acceptance-cases/08_auth.md "cargo test -p deve_cli identity_key_permissions_fail_closed_for_non_file -- --nocapture"
check_contains apps/cli/src/server/security.rs "enforce_owner_only_identity_key"
check_contains apps/cli/src/server/security.rs "permissions.set_mode(0o600)"
check_contains apps/cli/src/server/security.rs "identity_key_permissions_are_corrected_to_owner_only"
check_contains apps/cli/src/server/security.rs "identity_key_permissions_fail_closed_for_non_file"

# AUTH-001/002: runtime config is env-driven and production fails closed.
check_contains crates/core/src/security/auth/config.rs "ERROR: Production mode requires AUTH_SECRET and AUTH_PASS"
check_contains crates/core/src/security/auth/config.rs "AUTH_SECRET must be >= 32 bytes"
check_contains crates/core/src/security/auth/config.rs "AUTH_PASS must be a valid Argon2 PHC hash"
check_contains crates/core/src/security/auth/config.rs "AUTH_TOKEN_VERSION must be a valid u32 integer"
check_contains crates/core/src/security/auth/config.rs "missing_secret_or_password_fails_closed_in_production"
check_contains crates/core/src/security/auth/config.rs "invalid_auth_token_version_fails_closed"
check_absent crates/core/src/security/auth/config.rs 'expect("checked above")'
check_contains apps/cli/src/server/router.rs "WARNING: development-only auth defaults active"

# AUTH-009: JWT payload is minimized to sub, iat, exp, and version.
check_contains docs/acceptance-cases/08_auth.md "case_id: AUTH-009"
check_contains docs/acceptance-cases/08_auth.md "cargo test -p deve_core issue_token_preserves_subject -- --nocapture"
check_contains crates/core/src/security/auth/jwt.rs "sub"
check_contains crates/core/src/security/auth/jwt.rs "iat"
check_contains crates/core/src/security/auth/jwt.rs "exp"
check_contains crates/core/src/security/auth/jwt.rs "ver"
check_contains crates/core/src/security/auth/jwt.rs "subject: &str"
check_contains crates/core/src/security/auth/jwt.rs "sub: subject.to_string()"
check_contains crates/core/src/security/auth/jwt.rs "fn issue_token_preserves_subject()"
check_absent docs/acceptance-cases/08_auth.md "deve auth decode-jwt"

# AUTH-003/012: cookie session and status endpoint contract.
check_contains apps/cli/src/server/auth/handlers/session.rs ".http_only(true)"
check_contains apps/cli/src/server/auth/handlers/session.rs ".same_site(SameSite::Strict)"
check_contains apps/cli/src/server/auth/handlers/session.rs ".secure(https_enabled())"
check_contains apps/cli/src/server/auth/handlers/session.rs "https_enabled_invalid_value_fails_secure"
check_contains apps/cli/src/server/auth/handlers/session.rs "Login audit"
check_contains apps/cli/src/server/auth/handlers/session.rs "user_agent"
check_contains apps/cli/src/server/auth/handlers/session.rs "AuthStatusResponse::unauthenticated()"
check_contains apps/cli/src/server/router.rs ".route(\"/api/auth/status\", get(auth::handlers::status))"

# AUTH-014: anonymous localhost remains dev-only but uses a per-browser
# dev-session cookie for HTTP/WS grant binding.
check_contains docs/acceptance-cases/08_auth.md "case_id: AUTH-014"
check_contains docs/acceptance-cases/08_auth.md "anonymous_localhost_status_sets_dev_session_cookie"
check_contains docs/acceptance-cases/08_auth.md "anonymous_localhost_ws_uses_dev_session_cookie"
check_contains docs/acceptance-cases/08_auth.md "anonymous_localhost_auth_prefers_valid_jwt_over_dev_session_cookie"
check_contains docs/acceptance-cases/08_auth.md "rejects_forged_dev_session_cookie"
check_contains docs/acceptance-cases/08_auth.md "dev_session_cookie_is_bound_to_signing_secret"
check_contains apps/cli/src/server/auth/dev_session.rs "DEV_SESSION_COOKIE_NAME"
check_contains apps/cli/src/server/auth/dev_session.rs "deve_dev_session"
check_contains apps/cli/src/server/auth/dev_session.rs "hmac_sha256"
check_contains apps/cli/src/server/auth/browser_session.rs "AuthSessionId::from_cookie_token"
check_contains apps/cli/src/server/auth/browser_session.rs "AuthSessionId::from_dev_session_cookie"
check_contains apps/cli/src/server/auth/browser_session.rs "anonymous_localhost_auth_prefers_valid_jwt_over_dev_session_cookie"
check_contains apps/cli/src/server/auth/middleware.rs "browser_session::resolve_required"
check_contains apps/cli/src/server/ws/auth.rs "browser_session::resolve_required"
check_contains apps/cli/src/server/ws/mod.rs "admission.set_cookie()"

# AUTH-005: auth cookie extraction must match the exact token name and reject
# token_csrf/tokenBackup prefix traps.
check_contains docs/acceptance-cases/08_auth.md "case_id: AUTH-005"
check_contains apps/cli/src/server/auth/cookie.rs "token_csrf"
check_contains apps/cli/src/server/auth/cookie.rs "tokenBackup=bad"
check_contains apps/cli/src/server/auth/handlers/session.rs "fn status_rejects_token_cookie_prefixes()"

# AUTH-007: protected writes reject missing or invalid auth.
check_contains apps/cli/src/server/auth/middleware.rs "Json(AuthErrorResponse::new(code))"
check_contains apps/cli/src/server/auth/middleware.rs "development-only anonymous localhost auth bypass active"

# AUTH-008: login rate limiting must block repeated failures and fail closed
# when the brute-force guard is poisoned.
check_contains docs/acceptance-cases/08_auth.md "case_id: AUTH-008"
check_contains apps/cli/src/server/auth/middleware.rs "StatusCode::TOO_MANY_REQUESTS"
check_contains apps/cli/src/server/auth/handlers/login.rs "Login blocked (brute force)"
check_contains apps/cli/src/server/auth/brute_force.rs "BruteForceGuard lock poisoned; failing closed"
check_contains apps/cli/src/server/auth/brute_force/tests.rs "fn test_blocked_after_max_failures()"
check_contains apps/cli/src/server/auth/brute_force/tests.rs "fn poisoned_lock_blocks_ip_fail_closed()"

# AUTH-010: WebSocket handshake failures must be 401 structured JSON with
# AUTH_TOKEN_MISSING / token auth error codes.
check_contains docs/acceptance-cases/08_auth.md "case_id: AUTH-010"
check_contains apps/cli/src/server/ws/mod.rs "Json(AuthErrorResponse::new(code))"
check_contains apps/cli/src/server/ws/mod.rs "StatusCode::UNAUTHORIZED"
check_contains apps/cli/src/server/ws/mod.rs "async fn unauthorized_ws_response_is_structured_json()"
check_contains apps/cli/src/server/ws/auth.rs "AuthErrorCode::TokenMissing"

# AUTH-004/011: security headers and frontend session-expired state separation.
check_contains apps/cli/src/server/setup.rs "Wildcard CORS origin is forbidden"
check_contains apps/cli/src/server/setup.rs "development-only CORS allow list active"
check_contains apps/cli/src/server/auth/headers.rs "X-Content-Type-Options"
check_contains apps/cli/src/server/auth/headers.rs "X-Frame-Options"
check_contains apps/cli/src/server/auth/headers.rs "Content-Security-Policy"
check_contains apps/cli/src/server/auth/headers.rs "HeaderValue::from_static(CSP_POLICY)"
check_absent apps/cli/src/server/auth/headers.rs ".parse().unwrap()"
check_contains apps/web/src/api/auth_probe.rs "matches!(status, 401 | 403) || has_auth_error_code"
check_contains apps/web/src/api/connection.rs "try_set_connection_status(&signals, ConnectionStatus::Unauthorized)"
check_contains apps/web/src/api/connection.rs "set_status_and_revoke_writer_ready("
check_contains apps/web/src/components/disconnect_overlay.rs "ConnectionStatus::Unauthorized | ConnectionStatus::Connected => None"

echo "auth-baseline-check: ok"

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

# AUTH-001/002/009: runtime config is env-driven and production fails closed.
check_contains crates/core/src/security/auth/config.rs "ERROR: Production mode requires AUTH_SECRET and AUTH_PASS"
check_contains crates/core/src/security/auth/config.rs "AUTH_SECRET must be >= 32 bytes"
check_contains crates/core/src/security/auth/config.rs "AUTH_PASS must be a valid Argon2 PHC hash"
check_contains crates/core/src/security/auth/config.rs "missing_secret_or_password_fails_closed_in_production"
check_absent crates/core/src/security/auth/config.rs 'expect("checked above")'
check_contains apps/cli/src/server/router.rs "WARNING: development-only auth defaults active"
check_contains crates/core/src/security/auth/jwt.rs "sub"
check_contains crates/core/src/security/auth/jwt.rs "iat"
check_contains crates/core/src/security/auth/jwt.rs "exp"
check_contains crates/core/src/security/auth/jwt.rs "ver"
check_contains crates/core/src/security/auth/jwt.rs "subject: &str"
check_contains crates/core/src/security/auth/jwt.rs "sub: subject.to_string()"
check_absent docs/acceptance-cases/08_auth.md "deve auth decode-jwt"

# AUTH-003/005/012: cookie session and status endpoint contract.
check_contains apps/cli/src/server/auth/handlers/session.rs ".http_only(true)"
check_contains apps/cli/src/server/auth/handlers/session.rs ".same_site(SameSite::Strict)"
check_contains apps/cli/src/server/auth/handlers/session.rs ".secure(https_enabled())"
check_contains apps/cli/src/server/auth/handlers/session.rs "Login audit"
check_contains apps/cli/src/server/auth/handlers/session.rs "user_agent"
check_contains apps/cli/src/server/auth/cookie.rs "token_csrf"
check_contains apps/cli/src/server/auth/handlers/session.rs "AuthStatusResponse::unauthenticated()"
check_contains apps/cli/src/server/router.rs ".route(\"/api/auth/status\", get(auth::handlers::status))"

# AUTH-007/008/010: auth middleware, rate limiting, and WS handshake errors are structured.
check_contains apps/cli/src/server/auth/middleware.rs "Json(AuthErrorResponse::new(code))"
check_contains apps/cli/src/server/auth/middleware.rs "StatusCode::TOO_MANY_REQUESTS"
check_contains apps/cli/src/server/auth/handlers/login.rs "Login blocked (brute force)"
check_contains apps/cli/src/server/auth/brute_force.rs "BruteForceGuard lock poisoned; failing closed"
check_contains apps/cli/src/server/ws/mod.rs "Json(AuthErrorResponse::new(code))"
check_contains apps/cli/src/server/ws/mod.rs "StatusCode::UNAUTHORIZED"
check_contains apps/cli/src/server/auth/middleware.rs "development-only anonymous localhost auth bypass active"

# AUTH-004/011: security headers and frontend session-expired state separation.
check_contains apps/cli/src/server/setup.rs "Wildcard CORS origin is forbidden"
check_contains apps/cli/src/server/setup.rs "development-only CORS allow list active"
check_contains apps/cli/src/server/auth/headers.rs "X-Content-Type-Options"
check_contains apps/cli/src/server/auth/headers.rs "X-Frame-Options"
check_contains apps/cli/src/server/auth/headers.rs "Content-Security-Policy"
check_contains apps/cli/src/server/auth/headers.rs "HeaderValue::from_static(CSP_POLICY)"
check_absent apps/cli/src/server/auth/headers.rs ".parse().unwrap()"
check_contains apps/web/src/api/auth_probe.rs "matches!(status, 401 | 403) || has_auth_error_code"
check_contains apps/web/src/api/connection.rs ".try_set(signals.set_status, ConnectionStatus::Unauthorized)"
check_contains apps/web/src/components/disconnect_overlay.rs "ConnectionStatus::Unauthorized | ConnectionStatus::Connected => None"

echo "auth-baseline-check: ok"

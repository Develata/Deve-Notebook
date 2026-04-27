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

# AUTH-001/002/009: runtime config is env-driven and production fails closed.
check_contains crates/core/src/security/auth/config.rs "ERROR: Production mode requires AUTH_SECRET and AUTH_PASS"
check_contains crates/core/src/security/auth/config.rs "AUTH_SECRET must be >= 32 bytes"
check_contains apps/cli/src/server/router.rs "WARNING: Development mode with default credentials"
check_contains crates/core/src/security/auth/jwt.rs "sub"
check_contains crates/core/src/security/auth/jwt.rs "iat"
check_contains crates/core/src/security/auth/jwt.rs "exp"
check_contains crates/core/src/security/auth/jwt.rs "ver"

# AUTH-003/005/012: cookie session and status endpoint contract.
check_contains apps/cli/src/server/auth/handlers/session.rs ".http_only(true)"
check_contains apps/cli/src/server/auth/handlers/session.rs ".same_site(SameSite::Strict)"
check_contains apps/cli/src/server/auth/handlers/session.rs ".secure(https_enabled())"
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

# AUTH-004/011: security headers and frontend session-expired state separation.
check_contains apps/cli/src/server/auth/headers.rs "X-Content-Type-Options"
check_contains apps/cli/src/server/auth/headers.rs "X-Frame-Options"
check_contains apps/cli/src/server/auth/headers.rs "Content-Security-Policy"
check_contains apps/web/src/api/auth_probe.rs "matches!(status, 401 | 403) || has_auth_error_code"
check_contains apps/web/src/api/connection.rs "set_status.set(ConnectionStatus::Unauthorized);"
check_contains apps/web/src/components/disconnect_overlay.rs "ConnectionStatus::Connected | ConnectionStatus::Unauthorized"

echo "auth-baseline-check: ok"

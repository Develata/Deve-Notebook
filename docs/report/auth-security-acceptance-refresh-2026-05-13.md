# Auth Security Acceptance Refresh - 2026-05-13

## Scope

- Source of truth: `docs/plan/09_auth.md`.
- Acceptance cases: `AUTH-003` through `AUTH-010`, plus `AUTH-012`.
- Non-goal: redesign auth, add a CSRF-token subsystem, or change `docs/plan/`.

## Result

No runtime defect was found in this refresh.

One acceptance example was corrected: `AUTH-007` now targets the current protected HTTP write surface, `/api/sc/commit`, instead of the stale `/api/write` path.

## Live HTTP Checks

Environment:

- Data root: `/tmp/deve-auth-refresh-20260513-dfNd6I`.
- Static dir: `apps/web/dist`.
- Dev auth: explicit `serve --dev`.
- Cookie secure check: `HTTPS_ENABLED=true`.
- Curl used `--noproxy '*'` because the host proxy returned `502` for loopback requests.

Observed behavior:

- `GET /api/auth/status` without cookie returned `200 {"authenticated":false}`.
- `POST /api/auth/login` with `admin/admin` returned `200 {"success":true}`.
- Login response contained `Set-Cookie: token=...; HttpOnly; SameSite=Strict; Secure; Path=/`.
- `GET /api/auth/me` with `Cookie: token_csrf=bad` returned `401 {"code":"AUTH_TOKEN_MISSING"}`.
- `GET /ws` without cookie returned `401 {"code":"AUTH_TOKEN_MISSING"}`.
- `GET /api/node/role` with `Origin: http://evil.example` did not emit `Access-Control-Allow-Origin: *`.
- Repeated bad login attempts returned `401` for failed password and `429 {"success":false,"code":"AUTH_RATE_LIMITED"}` after the brute-force window was exhausted.
- Production startup without `AUTH_SECRET` / `AUTH_PASS` exited non-zero with `ERROR: Production mode requires AUTH_SECRET and AUTH_PASS`.
- `POST /api/sc/commit` with cross-site `Origin` and no auth cookie returned `401 {"code":"AUTH_TOKEN_MISSING"}`.

JWT payload decoded from the live login token contained only:

- `sub`
- `iat`
- `exp`
- `ver`

## Contract Notes

- CSRF protection is currently the plan-defined baseline: `SameSite=Strict` is the main browser defense; CORS stays explicit and fail-closed for wildcard origins; protected HTTP writes require a valid auth cookie.
- There is no separate CSRF token middleware in the current plan. This refresh does not introduce one.
- `/api/auth/status` remains the quiet public session probe and does not produce unauthenticated `401` noise.
- Unauthorized WS handshake remains a structured JSON HTTP rejection and does not enter repo/sync handshake.

## Verification

- `cargo test -p deve_core auth -- --nocapture`: passed.
- `cargo test -p deve_cli auth -- --nocapture`: passed.
- `cargo test -p deve_web auth_probe -- --nocapture`: passed.
- `cargo test -p deve_cli allowed_origins_from_env_fails_closed_on_wildcard_origin -- --nocapture`: passed.
- `cargo test -p deve_cli ws_endpoint_unauthorized_response_is_structured_json -- --nocapture`: passed.
- `cargo test -p deve_cli auth_status_endpoint_is_public_and_quiet_when_missing_token -- --nocapture`: passed.
- `bash scripts/check-auth-baseline.sh`: passed.
- `bash scripts/check-auth-unauthorized-state.sh`: passed.

## Decision

`Auth security acceptance refresh` is closed for the current queue. Next work should proceed to `Desktop Web shell browser smoke`.

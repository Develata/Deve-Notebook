# Dockerfile Plan Drift Decision

Date: 2026-05-16

## Scope

- Plan basis: `15_release.md#5-docker-deployment`.
- Code paths: `Dockerfile`, `scripts/check-release-baseline.sh`, GitHub Docker Smoke.
- Non-goal: redesign release workflow, add multi-arch publishing, or reopen native signing/store readiness.

## Decision

Keep the current direct locked Docker build.

Rationale:

- `Dockerfile` already builds the frontend first, embeds `apps/web/dist` into `deve_cli`, and runs `cargo build --release --locked --package deve_cli`.
- `scripts/check-release-baseline.sh` already rejects dependency recipe cache tooling in the Dockerfile.
- GitHub Docker Smoke has passed with the direct locked build path.
- Restoring dependency recipe cache tooling would add CI surface and reintroduce a path the current guard explicitly forbids.

## Changes

- Updated `docs/plan/15_release.md` to state that the Docker release baseline uses a locked direct release build.
- Kept dependency cache layers as optional future optimization, gated by locked CI and Docker smoke.
- Extended `scripts/check-release-baseline.sh` to guard the plan wording and prevent the old Dockerfile strategy from returning silently.

## Verification

Ran:

- `bash scripts/check-release-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `git diff --check`

Results:

- Release baseline: pass.
- Plan coverage: pass.
- Acceptance bindings: pass.
- Feature operation paths: pass.
- Diff hygiene: pass.

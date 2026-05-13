# release_quality_gates.md - Release 质量闸门链

## Metadata

- `Flow ID`: `flow.release.quality-gates`
- `Domain`: `release`
- `Related Feature Chapters`: `docs/features/15_release.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `REL-003`, `REL-007`, `REL-008`, `TECH-001`, `PERF-001`

## Operations

### `op.release.quality.start-test-job`

- `Name`: `Start Release Test Job`
- `Surface`: `github-actions`
- `Trigger`: release workflow dispatches the `test` job
- `Preconditions`: checkout and Rust toolchain setup succeed
- `Immediate Result`: release enters the verification stage before publish
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.quality.run-clippy`

- `Name`: `Run Clippy Gate`
- `Surface`: `github-actions`
- `Trigger`: release test job reaches lint stage
- `Preconditions`: workspace dependencies are available
- `Immediate Result`: warning-level regressions block publish
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.quality.check-web-wasm`

- `Name`: `Check Web WASM Target`
- `Surface`: `github-actions`
- `Trigger`: release test job reaches browser build compatibility stage
- `Preconditions`: `wasm32-unknown-unknown` target is installed
- `Immediate Result`: protocol or shared-core references to backend-only modules fail before publish
- `Application Entry`: `.github/workflows/release.yml`, `apps/web/`

### `op.release.quality.audit-dependencies`

- `Name`: Audit Dependency Advisories
- `Surface`: `github-actions`, `local-or-ci-script`
- `Trigger`: release test job or maintainer release preflight reaches dependency audit stage
- `Preconditions`: `cargo-audit` and `npm` are available, or local diagnostic-only skip is acceptable
- `Immediate Result`: high-risk dependency advisories block release when required mode is enabled
- `Application Entry`: `.github/workflows/release.yml`, `scripts/check-release-audit-gate.sh`

### `op.release.quality.run-tests`

- `Name`: `Run Test Gate`
- `Surface`: `github-actions`
- `Trigger`: release test job reaches test stage
- `Preconditions`: build artifacts can be produced
- `Immediate Result`: failing tests stop later release steps
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.quality.run-runtime-happy-path-smoke`

- `Name`: `Run Runtime Happy Path Smoke`
- `Surface`: `local-or-ci-script`
- `Trigger`: maintainer verifies the runtime write/read path after architecture changes
- `Preconditions`: workspace tests can build
- `Immediate Result`: temporary repo WS create/edit/open/history path is verified
- `Application Entry`: `scripts/smoke-runtime-happy-path.sh`

### `op.release.quality.run-runtime-recovery-smoke`

- `Name`: `Run Runtime Recovery Smoke`
- `Surface`: `local-or-ci-script`
- `Trigger`: maintainer verifies recovery behavior after runtime or sync changes
- `Preconditions`: workspace tests can build
- `Immediate Result`: degraded write gates, stale sync-scope cleanup, reconnect gates, and auth-probe separation are verified
- `Application Entry`: `scripts/smoke-runtime-recovery-path.sh`

## Response Flow

1. Release dispatch enters the `test` job.
2. Instruction interface is the CI job surface and its ordered verification steps.
3. Flow coordination enforces lint, web WASM compatibility, dependency audit, and test gates before publish; runtime happy-path and recovery smokes remain explicit local/CI script gates.
4. Execution domains are CI release logic, quality gates, and runtime budget policy.

## Notes

- Current workflow explicitly models `clippy`, `deve_web` WASM check, dependency audit, and `cargo test`; runtime smoke scripts are local/CI gates that can be run before broader release verification.
- Main objects: `quality::gate`, `ci::workflow`, `runtime::budget`.

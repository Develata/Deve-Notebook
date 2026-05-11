# release_quality_gates.md - Release 质量闸门链

## Metadata

- `Flow ID`: `flow.release.quality-gates`
- `Domain`: `release`
- `Related Feature Chapters`: `docs/features/15_release.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `REL-003`, `REL-007`, `TECH-001`, `PERF-001`

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

## Response Flow

1. Release dispatch enters the `test` job.
2. Instruction interface is the CI job surface and its ordered verification steps.
3. Flow coordination enforces lint, web WASM compatibility, and test gates before publish; runtime happy-path smoke remains an explicit local/CI script gate.
4. Execution domains are CI release logic, quality gates, and runtime budget policy.

## Notes

- Current workflow explicitly models `clippy`, `deve_web` WASM check, and `cargo test`; the runtime happy-path smoke is a local/CI script that can be run before broader release verification.
- Main objects: `quality::gate`, `ci::workflow`, `runtime::budget`.

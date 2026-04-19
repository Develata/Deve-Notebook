# release_quality_gates.md - Release 质量闸门链

## Metadata

- `Flow ID`: `flow.release.quality-gates`
- `Domain`: `release`
- `Related Feature Chapters`: `docs/features/15_release.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `REL-003`, `TECH-001`, `PERF-001`

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

### `op.release.quality.run-tests`

- `Name`: `Run Test Gate`
- `Surface`: `github-actions`
- `Trigger`: release test job reaches test stage
- `Preconditions`: build artifacts can be produced
- `Immediate Result`: failing tests stop later release steps
- `Application Entry`: `.github/workflows/release.yml`

## Response Flow

1. Release dispatch enters the `test` job.
2. Instruction interface is the CI job surface and its ordered verification steps.
3. Flow coordination enforces lint and test gates before publish.
4. Execution domains are CI release logic, quality gates, and runtime budget policy.

## Notes

- Current workflow explicitly models `clippy` and `cargo test`; wider release checklists still remain plan-visible review policy.
- Main objects: `quality::gate`, `ci::workflow`, `runtime::budget`.

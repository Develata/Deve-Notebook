# tech_stack_runtime_budget_check.md - 技术栈运行预算检查链

## Metadata

- `Flow ID`: `flow.tech-stack.runtime-budget-check`
- `Domain`: `tech-stack`
- `Related Feature Chapters`: `docs/features/14_tech_stack.md`, `docs/features/15_release.md`
- `Related Acceptance Cases`: `PERF-001`, `REL-003`, `TECH-001`

## Operations

### `op.tech.budget.select-profile`

- `Name`: `Select Low-Spec Profile`
- `Surface`: `env-or-config`
- `Trigger`: deployer chooses low-resource runtime profile
- `Preconditions`: configuration system can read the selected profile
- `Immediate Result`: heavy capabilities are expected to be gated
- `Application Entry`: `DEVE_PROFILE=low-spec`, `config.toml`

### `op.tech.budget.run-check`

- `Name`: `Run Runtime Budget Check`
- `Surface`: `cli-or-ci`
- `Trigger`: maintainer runs budget or release checks
- `Preconditions`: repo is checked out and dependencies are available
- `Immediate Result`: budget contract drift and low-spec config drift are caught before release
- `Application Entry`: `scripts/plan-coverage.sh`, `scripts/check-perf-budget-baseline.sh`, `.github/workflows/release.yml`
- `Baseline`: `PERF-001` open-doc / edit-ack / cold-mount / RSS contract entries are bound by `cargo run -p deve_baseline -- perf-budget`; measured regression gates are added by the later benchmark harness.

### `op.tech.budget.review-heavy-feature`

- `Name`: `Review Heavy Feature Gate`
- `Surface`: `review`
- `Trigger`: feature proposes search, graph, AI, or plugin-heavy behavior
- `Preconditions`: expected memory and runtime cost are known
- `Immediate Result`: feature is gated, deferred, or scoped for low-spec
- `Application Entry`: plan chapter 14, release checklist

## Response Flow

1. Deployer or maintainer selects a runtime profile or runs verification.
2. Instruction interface is config loading, CI job, or review checklist.
3. Flow coordination compares runtime cost against low-spec policy.
4. Execution domains are tech-stack constraints, config, and release gates.

## Notes

- Runtime budget is a release-blocking constraint, not a cosmetic metric.
- Main objects: `runtime::budget`, `budget::gate`, `config::runtime`.

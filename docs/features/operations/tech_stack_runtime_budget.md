# tech_stack_runtime_budget.md - 技术栈与运行预算检查链

## Metadata

- `Flow ID`: `flow.tech-stack.runtime-budget`
- `Domain`: `tech-stack`
- `Related Feature Chapters`: `docs/features/14_tech_stack.md`, `docs/features/15_release.md`
- `Related Acceptance Cases`: `TECH-001`, `PERF-001`, `REL-003`

## Operations

### `op.tech.inspect-dependencies`

- `Name`: `Inspect Dependency Set`
- `Surface`: `repo-files`
- `Trigger`: maintainer reads `Cargo.toml`, `apps/web/package.json`, or lockfiles
- `Preconditions`: dependency manifests are available
- `Immediate Result`: tech-stack choices can be compared with plan
- `Application Entry`: `Cargo.toml`, `apps/web/package.json`

### `op.tech.run-budget-check`

- `Name`: `Run Runtime Budget Check`
- `Surface`: `cli-or-ci`
- `Trigger`: run low-spec or CI verification checks
- `Preconditions`: repo is checked out and dependencies are available
- `Immediate Result`: heavy features remain gated for low-resource target
- `Application Entry`: `scripts/plan-coverage.sh`, `.github/workflows/release.yml`

### `op.tech.inspect-platform-maturity`

- `Name`: `Inspect Platform Maturity`
- `Surface`: `docs-or-about-surface`
- `Trigger`: maintainer checks whether a platform is stable, planned, or future-only
- `Preconditions`: release and tech-stack docs are current
- `Immediate Result`: unfinished platforms are not presented as stable release targets
- `Application Entry`: `docs/features/14_tech_stack.md`, `docs/features/15_release.md`

## Response Flow

1. User or maintainer inspects tech-stack, runtime budget, or platform maturity.
2. Instruction interface is repo-file inspection, CI script entry, or visible release metadata.
3. Flow coordination compares current dependencies and release surfaces with the plan.
4. Execution domains are build tooling, release automation, config/runtime budget, and dependency policy.

## Notes

- This flow protects the 768 MB VPS target from accidental dependency creep.
- Main objects: `tech::dependency`, `runtime::budget`, `platform::maturity`.

# tech_stack_dependency_policy.md - 技术栈依赖策略链

## Metadata

- `Flow ID`: `flow.tech-stack.dependency-policy`
- `Domain`: `tech-stack`
- `Related Feature Chapters`: `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `TECH-001`, `PERF-001`

## Operations

### `op.tech.deps.inspect-manifest`

- `Name`: `Inspect Dependency Manifest`
- `Surface`: `repo-files`
- `Trigger`: maintainer reads `Cargo.toml` or `apps/web/package.json`
- `Preconditions`: dependency manifests are available
- `Immediate Result`: direct dependencies can be compared with the plan
- `Application Entry`: `Cargo.toml`, `apps/web/package.json`

### `op.tech.deps.inspect-lockfile`

- `Name`: `Inspect Dependency Lockfile`
- `Surface`: `repo-files`
- `Trigger`: maintainer checks lockfile drift or dependency expansion
- `Preconditions`: lockfiles are available
- `Immediate Result`: transitive dependency growth is visible
- `Application Entry`: `Cargo.lock`, `apps/web/package-lock.json`

### `op.tech.deps.evaluate-new`

- `Name`: `Evaluate New Dependency`
- `Surface`: `review`
- `Trigger`: proposed change adds or upgrades a dependency
- `Preconditions`: target dependency purpose and footprint are known
- `Immediate Result`: dependency is accepted, rejected, or deferred by policy
- `Application Entry`: review checklist, plan chapter 14

## Response Flow

1. Maintainer inspects dependency manifests or proposed dependency changes.
2. Instruction interface is repo inspection or review gate.
3. Flow coordination checks plan fit, low-resource impact, and replacement cost.
4. Execution domain is tech-stack dependency policy.

## Notes

- This flow protects the 768 MB VPS target from dependency creep.
- Main objects: `tech::dependency`, `dependency::policy`, `runtime::budget`.

# release_ci.md - Release 与 CI 交付链

## Metadata

- `Flow ID`: `flow.release.ci`
- `Domain`: `release`
- `Related Feature Chapters`: `docs/features/15_release.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `REL-001`, `REL-002`, `REL-003`, `TECH-001`, `PERF-001`

## Operations

### `op.release.ci.push-tag`

- `Name`: `Push Release Tag`
- `Surface`: `git`
- `Trigger`: push tag matching `v*`
- `Preconditions`: release tag exists and GitHub Actions is enabled
- `Immediate Result`: `.github/workflows/release.yml` starts the release workflow
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.ci.run-quality-gates`

- `Name`: `Run Release Quality Gates`
- `Surface`: `github-actions`
- `Trigger`: release workflow `test` job starts
- `Preconditions`: checkout and Rust toolchain install succeeded
- `Immediate Result`: clippy and tests run before publish
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.ci.publish-container`

- `Name`: `Publish Release Container`
- `Surface`: `github-actions`
- `Trigger`: `docker` job starts after successful tests
- `Preconditions`: GHCR login succeeded
- `Immediate Result`: Docker image is built and pushed with semver and `latest` tags
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.ci.inspect-channel-coverage`

- `Name`: `Inspect Channel Coverage`
- `Surface`: `repo-files`
- `Trigger`: maintainer verifies expected release workflows
- `Preconditions`: `.github/workflows/` is readable
- `Immediate Result`: current repo shows `release.yml`; `nightly.yml` and `speckit-sync-check.yml` are absent
- `Application Entry`: `.github/workflows/`

## Response Flow

1. User pushes a release tag or inspects workflow coverage.
2. Instruction interface is GitHub Actions workflow dispatch or repo file inspection.
3. Flow coordination is split between release test job, Docker publish job, and workflow coverage check.
4. Execution domains are release automation, build tooling, Docker/GHCR, and spec-sync policy.

## Notes

- Current code side implements tag-driven release via `release.yml`.
- Current code side does not contain `nightly.yml` or `speckit-sync-check.yml`; the architecture diff should mark this flow as drift until those workflows are added or plan text is revised.
- Main objects: `release::tag`, `ci::workflow`, `container::image`, `spec::sync-state`.

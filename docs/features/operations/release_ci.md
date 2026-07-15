# release_ci.md - Release 与 CI 交付链

## Metadata

- `Flow ID`: `flow.release.ci`
- `Domain`: `release`
- `Related Feature Chapters`: `docs/features/15_release.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `REL-001`, `REL-002`, `REL-003`, `TECH-001`, `PERF-001`
- `Summary-Only`: `yes`

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
- `Trigger`: maintainer dispatches `release-candidate.yml`
- `Preconditions`: checkout and Rust toolchain install succeeded
- `Immediate Result`: full baseline、clippy、WASM check、tests 与 security producer 在任何 tag 前运行
- `Application Entry`: `.github/workflows/release-candidate.yml`

### `op.release.ci.publish-container`

- `Name`: `Publish Release Container`
- `Surface`: `github-actions`
- `Trigger`: tag promotion job has verified its annotated-tag-bound sealed bundle
- `Preconditions`: candidate/receipts/tag identity passed and GHCR login succeeded
- `Immediate Result`: aggregate-bound Docker archive is loaded and identity-checked; version is pushed, while only a strictly newer stable release may update `latest`
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.ci.inspect-channel-coverage`

- `Name`: `Inspect Channel Coverage`
- `Surface`: `repo-files`
- `Trigger`: maintainer verifies expected release workflows
- `Preconditions`: `.github/workflows/` is readable
- `Immediate Result`: repo shows candidate build/smoke、aggregate seal、tag promotion 与 reusable native track 的唯一职责
- `Application Entry`: `.github/workflows/`

## Response Flow

1. Maintainer dispatches an exact-HEAD candidate, then dispatches its aggregate after all platforms pass.
2. Only after tag-ready succeeds does the maintainer push an annotated tag binding the aggregate run ID.
3. Tag flow verifies and promotes sealed bytes; workflow inspection proves no tag-time rebuild path exists.
4. Execution domains are release automation, target-host evidence, build tooling, Docker/GHCR, and spec-sync policy.

## Notes

- This file is a summary flow. Use the split release flows as the authoritative implementation read path.
- Current code side separates pre-tag candidate build/sealing from tag-driven promotion in `release.yml`.
- `nightly.yml` and `speckit-sync-check.yml` are intentionally outside the current release / CI baseline.
- Main objects: `release::tag`, `ci::workflow`, `container::image`.

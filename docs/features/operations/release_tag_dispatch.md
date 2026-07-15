# release_tag_dispatch.md - Release 标签分发链

## Metadata

- `Flow ID`: `flow.release.tag-dispatch`
- `Domain`: `release`
- `Related Feature Chapters`: `docs/features/15_release.md`
- `Related Acceptance Cases`: `REL-001`, `REL-003`

## Operations

### `op.release.tag.push-semver`

- `Name`: `Push Semver Release Tag`
- `Surface`: `git`
- `Trigger`: maintainer pushes tag matching `v*`
- `Preconditions`: release version is frozen, exact-HEAD candidate/receipt aggregate succeeded, the tag is annotated with exactly one `Deve-Acceptance-Aggregate-Run: <run-id>` trailer, and repository is writable
- `Immediate Result`: tag becomes visible to GitHub Actions
- `Application Entry`: `git tag -a vX.Y.Z -m "Deve-Acceptance-Aggregate-Run: <run-id>" && git push origin vX.Y.Z`

### `op.release.tag.match-trigger`

- `Name`: `Match Release Trigger Rule`
- `Surface`: `workflow-config`
- `Trigger`: GitHub evaluates the pushed ref
- `Preconditions`: `.github/workflows/release.yml` is present
- `Immediate Result`: the broad `v*` trigger enters one promotion orchestrator, whose first gate rejects non-SemVer refs before checkout and whose post-checkout gate exact-matches the tag against workspace/Desktop/Mobile and the explicitly bound sealed candidate before publish; no build is permitted after the tag
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.tag.observe-dispatch`

- `Name`: `Observe Workflow Dispatch`
- `Surface`: `github-actions`
- `Trigger`: release workflow is created after tag push
- `Preconditions`: Actions is enabled for the repository
- `Immediate Result`: deployer can see the release pipeline start from the tag
- `Application Entry`: GitHub Actions run list

## Response Flow

1. Maintainer pushes an annotated semver release tag whose message binds one aggregate run ID.
2. Instruction interface is git ref delivery plus workflow trigger matching.
3. Flow coordination validates SemVer before repository checkout, exact-matches the checked-out versions including prerelease/build metadata, downloads the explicitly bound aggregate run, and promotes only the sealed bytes.
4. Execution domains are release trigger policy and CI workflow dispatch.

## Notes

- This flow defines the sole tag entry gate before any promotion or public mutation starts; all builds already occurred in `release-candidate.yml`, and `.github/workflows/release-native.yml` is build/smoke-only callable infrastructure rather than a second tag entry.
- Main objects: `release::tag`, `ci::workflow`, `release::channel`.

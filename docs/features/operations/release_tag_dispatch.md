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
- `Preconditions`: release version is chosen and repository is writable
- `Immediate Result`: tag becomes visible to GitHub Actions
- `Application Entry`: `git push origin vX.Y.Z`

### `op.release.tag.match-trigger`

- `Name`: `Match Release Trigger Rule`
- `Surface`: `workflow-config`
- `Trigger`: GitHub evaluates the pushed ref
- `Preconditions`: `.github/workflows/release.yml` is present
- `Immediate Result`: the broad `v*` trigger enters one orchestrator, whose first gate rejects non-SemVer refs before checkout/build/publish; native delivery has no independent tag trigger
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.tag.observe-dispatch`

- `Name`: `Observe Workflow Dispatch`
- `Surface`: `github-actions`
- `Trigger`: release workflow is created after tag push
- `Preconditions`: Actions is enabled for the repository
- `Immediate Result`: deployer can see the release pipeline start from the tag
- `Application Entry`: GitHub Actions run list

## Response Flow

1. Maintainer pushes a semver release tag.
2. Instruction interface is git ref delivery plus workflow trigger matching.
3. Flow coordination validates SemVer before repository checkout, decides whether the pushed ref may continue as a release run, and keeps reusable native delivery behind the orchestrator's quality and Docker jobs.
4. Execution domains are release trigger policy and CI workflow dispatch.

## Notes

- This flow defines the sole release entry gate before any build or publish step starts; `.github/workflows/release-native.yml` is callable infrastructure, not a second tag entry.
- Main objects: `release::tag`, `ci::workflow`, `release::channel`.

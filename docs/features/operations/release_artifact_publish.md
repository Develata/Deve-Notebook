# release_artifact_publish.md - Release 制品发布链

## Metadata

- `Flow ID`: `flow.release.artifact-publish`
- `Domain`: `release`
- `Related Feature Chapters`: `docs/features/15_release.md`
- `Related Acceptance Cases`: `REL-001`, `REL-002`, `REL-003`

## Operations

### `op.release.publish.login-ghcr`

- `Name`: `Login GHCR`
- `Surface`: `github-actions`
- `Trigger`: release workflow enters the `docker` job
- `Preconditions`: `test` job succeeded and package write permission exists
- `Immediate Result`: workflow can push release images to GHCR
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.publish.compute-tags`

- `Name`: `Compute Release Image Tags`
- `Surface`: `github-actions`
- `Trigger`: Docker metadata step runs
- `Preconditions`: release tag is available
- `Immediate Result`: semver and `latest` tags are derived for the image
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.publish.push-image`

- `Name`: `Build And Push Release Image`
- `Surface`: `github-actions`
- `Trigger`: docker build-push step runs
- `Preconditions`: registry auth and metadata steps succeeded
- `Immediate Result`: release container artifact is published
- `Application Entry`: `.github/workflows/release.yml`

## Response Flow

1. Release workflow enters the publish stage after quality gates pass.
2. Instruction interface is the docker job and its ordered steps.
3. Flow coordination authenticates registry access, computes tags, and pushes the artifact.
4. Execution domains are release automation, registry auth, and container delivery.

## Notes

- The current baseline only requires Docker/GHCR artifact publish, not multi-channel package distribution.
- Main objects: `release::artifact`, `container::image`, `release::channel`.

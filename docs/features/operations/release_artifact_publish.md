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

### `op.release.publish.smoke-candidate-image`

- `Name`: `Build And Smoke Candidate Image`
- `Surface`: `github-actions`
- `Trigger`: docker build step creates one local candidate image
- `Preconditions`: registry auth and metadata steps succeeded
- `Immediate Result`: runtime/login and multi-client browser smokes pass against the same recorded image ID without rebuilding
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.publish.push-image`

- `Name`: `Tag Push And Verify Release Image`
- `Surface`: `github-actions`
- `Trigger`: exact candidate image smokes succeed
- `Preconditions`: candidate image ID remains unchanged
- `Immediate Result`: version and latest tags are pushed and both remote references resolve to the same manifest digest
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.publish.build-native-artifacts`

- `Name`: `Build Required Native Artifacts`
- `Surface`: `github-actions-reusable-workflow`
- `Trigger`: orchestrator Docker job succeeds and calls native delivery
- `Preconditions`: release quality gates and Docker publish succeeded
- `Immediate Result`: Windows, macOS, and Android jobs upload internal workflow artifacts without creating a GitHub Release
- `Application Entry`: `.github/workflows/release-native.yml`

### `op.release.publish.attach-native-release`

- `Name`: `Attach Complete Native Artifact Set`
- `Surface`: `github-actions`
- `Trigger`: all required native build jobs succeed
- `Preconditions`: Windows, macOS, and Android workflow artifacts are available
- `Immediate Result`: one publish job rejects any downloaded file beyond the exact four-file allowlist, rejects rerun mutation of an already-public Release, uploads the validated set to a draft, verifies the remote asset set, and only then publishes one GitHub Release
- `Application Entry`: `.github/workflows/release-native.yml`

## Response Flow

1. Release workflow enters the publish stage after quality gates pass.
2. Instruction interface is the docker job and its ordered steps.
3. Flow coordination authenticates registry access, computes tags, builds and smokes one candidate image, pushes and verifies its two remote tags, calls native delivery, waits for every required native build, and publishes the native set once.
4. Execution domains are release automation, registry auth, container delivery, native package build, and GitHub Release publication.

## Notes

- Docker/GHCR publish precedes native delivery in the approved first-tag minimal orchestration. A later native, manifest, upload, or API verification failure leaves no public GitHub Release; an incomplete draft and the GHCR image require explicit recovery and must not be reported as a complete release.
- Main objects: `release::artifact`, `container::image`, `release::channel`.

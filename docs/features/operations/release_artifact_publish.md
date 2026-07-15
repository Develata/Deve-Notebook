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
- `Trigger`: promotion job has verified sealed candidate bytes and exact tag binding
- `Preconditions`: tag-ready receipts passed and package write permission exists
- `Immediate Result`: workflow can push release images to GHCR
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.publish.compute-tags`

- `Name`: `Compute Release Image Tags`
- `Surface`: `github-actions`
- `Trigger`: promotion derives an injective Docker-safe tag from the exact SemVer
- `Preconditions`: release tag is available
- `Immediate Result`: full SemVer remains in manifest/GitHub Release, `+build` maps to `_build_` for Docker, and prerelease omits `latest`
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.publish.smoke-candidate-image`

- `Name`: `Build Smoke And Seal Candidate Image`
- `Surface`: `github-actions`
- `Trigger`: pre-tag candidate workflow creates one local candidate image
- `Preconditions`: candidate quality gates succeeded
- `Immediate Result`: runtime/login, product and P2P smokes pass against one image ID; its archive, hash, SBOM and attestations are sealed without rebuilding
- `Application Entry`: `.github/workflows/release-candidate.yml`

### `op.release.publish.push-image`

- `Name`: `Tag Push And Verify Release Image`
- `Surface`: `github-actions`
- `Trigger`: tag workflow loads the aggregate-bound candidate archive
- `Preconditions`: candidate manifest, checksums, HEAD, version and image ID re-verify
- `Immediate Result`: immutable version is pushed; a strictly newer stable release also updates `latest`, and both stable references resolve to the same manifest digest
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.publish.build-native-artifacts`

- `Name`: `Build Required Native Artifacts`
- `Surface`: `github-actions-reusable-workflow`
- `Trigger`: pre-tag candidate orchestrator calls native build-only delivery
- `Preconditions`: candidate quality gates succeeded
- `Immediate Result`: Windows, macOS, and Android jobs upload exact candidate artifacts without creating a GitHub Release
- `Application Entry`: `.github/workflows/release-native.yml`

### `op.release.publish.attach-native-release`

- `Name`: `Attach Complete Native Artifact Set`
- `Surface`: `github-actions`
- `Trigger`: annotated tag binds a successful aggregate containing all required native artifacts and receipts
- `Preconditions`: the explicit aggregate run contains the sealed Windows, macOS, Android, SBOM and checksum assets
- `Immediate Result`: one promotion job rejects bytes or names beyond the sealed manifest, rejects rerun mutation of an already-public Release, uploads the unchanged set to a draft, verifies remote SHA-256, and only then publishes one GitHub Release
- `Application Entry`: `.github/workflows/release.yml`

## Response Flow

1. Candidate workflow builds/smokes/seals before the tag; aggregate independently validates the exact run.
2. Instruction interface is the annotated tag and its single promotion job.
3. Flow coordination first builds, smokes, hashes and attests one pre-tag candidate; after a tag, it loads that exact archive, pushes and verifies its remote tags, uploads the unchanged native set, and publishes once.
4. Execution domains are release automation, registry auth, container delivery, native package build, and GitHub Release publication.

## Notes

- Candidate native delivery completes before tag creation. Promotion remains non-atomic across GHCR and GitHub Release; an incomplete draft and any already-pushed GHCR image require explicit recovery and must not be reported as a complete release.
- Main objects: `release::artifact`, `container::image`, `release::channel`.

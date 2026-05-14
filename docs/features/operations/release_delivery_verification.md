# release_delivery_verification.md - Release 交付验证链

## Metadata

- `Flow ID`: `flow.release.delivery-verification`
- `Domain`: `release`
- `Related Feature Chapters`: `docs/features/15_release.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `REL-001`, `REL-002`, `REL-003`

## Operations

### `op.release.verify.inspect-workflow-surface`

- `Name`: `Inspect Workflow Surface`
- `Surface`: `repo-files`
- `Trigger`: maintainer checks release automation scope
- `Preconditions`: `.github/workflows/` is readable
- `Immediate Result`: repo shows `release.yml` as the current required release workflow
- `Application Entry`: `.github/workflows/`

### `op.release.verify.inspect-release-channel`

- `Name`: `Inspect Release Channel Surface`
- `Surface`: `ci-or-registry-metadata`
- `Trigger`: maintainer verifies naming and channel exposure
- `Preconditions`: release workflow and Docker metadata are readable
- `Immediate Result`: stable release channel remains distinguishable from future channels
- `Application Entry`: `.github/workflows/release.yml`

### `op.release.verify.verify-container-delivery`

- `Name`: `Verify Container Delivery`
- `Surface`: `docker`
- `Trigger`: deployer validates published server image
- `Preconditions`: Docker runtime and release image are available
- `Immediate Result`: current container delivery path can be smoke-tested
- `Application Entry`: `Dockerfile`, `docker-compose.yml`

## Response Flow

1. Maintainer or deployer verifies the release surface after publish or before rollout.
2. Instruction interface is workflow inspection, artifact metadata, or Docker invocation.
3. Flow coordination checks current delivery surface instead of assuming publish success.
4. Execution domains are workflow inventory, release channels, and container delivery.

## Notes

- This flow keeps "release completed" separate from "delivery verified".
- Main objects: `delivery::verification`, `release::artifact`, `release::channel`.

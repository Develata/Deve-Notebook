# tech_stack_platform_release_channel.md - 平台成熟度与发布通道链

## Metadata

- `Flow ID`: `flow.tech-stack.platform-release-channel`
- `Domain`: `tech-stack`
- `Related Feature Chapters`: `docs/features/14_tech_stack.md`, `docs/features/15_release.md`
- `Related Acceptance Cases`: `REL-001`, `REL-002`, `REL-003`, `TECH-001`

## Operations

### `op.tech.platform.inspect-matrix`

- `Name`: `Inspect Platform Matrix`
- `Surface`: `docs-or-about-surface`
- `Trigger`: maintainer checks whether a platform is stable, planned, or future-only
- `Preconditions`: support matrix documentation is current
- `Immediate Result`: unfinished platforms are not presented as stable targets
- `Application Entry`: `docs/features/14_tech_stack.md`, `docs/features/15_release.md`

### `op.tech.platform.verify-native-process-adapter`

- `Name`: `Verify Native Process Adapter Gate`
- `Surface`: `cli-or-ci`
- `Trigger`: maintainer validates desktop/mobile native shell progress
- `Preconditions`: native process adapter gate remains explicit and default build is unchanged
- `Immediate Result`: native shell progress is visible without advertising Tauri packaging or child-process runtime as default capability
- `Application Entry`: `scripts/check-native-track-boundary.sh`

### `op.tech.platform.verify-release-channel`

- `Name`: `Verify Release Channel`
- `Surface`: `ci-or-dist`
- `Trigger`: release artifact or tag is prepared
- `Preconditions`: release naming and channel rules are known
- `Immediate Result`: stable, preview, and future channels remain distinguishable
- `Application Entry`: `.github/workflows/release.yml`, `dist/`

### `op.tech.platform.verify-container`

- `Name`: `Verify Container Delivery`
- `Surface`: `docker`
- `Trigger`: deployer or CI validates server container delivery
- `Preconditions`: Docker environment and release image are available
- `Immediate Result`: current Web / Server / Docker delivery surface is verifiable
- `Application Entry`: `Dockerfile`, `docker-compose.yml`, `.github/workflows/release.yml`

## Response Flow

1. Maintainer or deployer checks platform status or release channel.
2. Instruction interface is documentation, native boundary check, CI metadata, or Docker invocation.
3. Flow coordination prevents future platforms from being advertised as stable.
4. Execution domains are platform matrix, native gate status, and release automation.

## Notes

- This flow separates current delivery surfaces from future client plans.
- Main objects: `platform::maturity`, `native::process_adapter_gate`, `release::channel`, `release::artifact`.

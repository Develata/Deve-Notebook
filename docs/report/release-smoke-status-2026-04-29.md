# Release Smoke Status 2026-04-29

This report records the release/runtime smoke status after the native adapter
boundary and graph projection surface batches.

## Summary

- Overall status: code gates are green; Docker release smoke remains blocked by
  host Docker availability.
- Code gate status: passing.
- Docker release smoke status: environment-blocked, not a code failure.
- Worktree note: `.codex` remained untracked and was not part of validation.

## Passing Gates

| Gate | Command | Result |
|---|---|---|
| Format | `cargo fmt --check` | Passed |
| Graph baseline | `scripts/check-graph-baseline.sh` | Passed |
| Release baseline | `scripts/check-release-baseline.sh` | Passed |
| Graph core tests | `cargo test -p deve_core --lib graph_projection -- --nocapture` | Passed |
| Graph CLI tests | `cargo test -p deve_cli graph -- --nocapture` | Passed |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | Passed |
| Full test suite | `cargo test` | Passed |

## Docker Smoke

Attempted prerequisite check:

```bash
docker info
```

Observed result:

```text
The command 'docker' could not be found in this WSL 2 distro.
We recommend to activate the WSL integration in Docker Desktop settings.
```

Attempted required smoke:

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 \
DEVE_DOCKER_SMOKE_PORT=3102 \
scripts/smoke-docker-release.sh
```

Observed result:

```text
docker-release-smoke: docker daemon is not reachable
```

Assessment: REL-002 remains blocked before Docker build/run starts. No release
image, container, or runtime endpoint failure was observed.

## Remaining Release Follow-Up

- Enable Docker Desktop WSL integration or provide another Docker-compatible
  daemon in this WSL environment.
- Re-run `DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_PORT=3102 scripts/smoke-docker-release.sh`.
- Keep `cargo test` / `cargo clippy --all-targets --all-features -- -D warnings`
  as the code gates before any release tag.

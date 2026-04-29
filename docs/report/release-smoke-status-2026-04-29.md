# Release Smoke Status 2026-04-29

This report records the release/runtime smoke status after the Git push
blocker/remote polish batch.

## Summary

- Overall status: code gates are green; Docker image build now passes through a
  Windows Docker fallback, but full container endpoint smoke remains blocked by
  the agent's WSL Docker integration.
- Code gate status: passing.
- Docker release smoke status: image build passed, endpoint verification
  environment-blocked, not a code failure.
- Latest rerun: after commit `10d68d89` (`Add git mirror failure metadata`).
- Worktree note: `.codex` remained untracked and was not part of validation.

## Passing Gates

| Gate | Command | Result |
|---|---|---|
| Format | `cargo fmt --check` | Passed |
| Diff whitespace | `git diff --check` | Passed |
| Git push CLI output | `cargo test -p deve_cli push_report_lines -- --nocapture` | Passed |
| Git push Web notice | `cargo test -p deve_web git_push -- --nocapture` | Passed |
| Plan coverage | `scripts/plan-coverage.sh` | Passed, 0 blocking violations |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | Passed |
| Full test suite | `cargo test` | Passed |

## Docker Smoke

Attempted prerequisite check:

```bash
docker info
```

Observed result after the previous rerun:

```text
The command 'docker' could not be found in this WSL 2 distro.
We recommend to activate the WSL integration in Docker Desktop settings.
```

Additional 2026-04-29 checks:

```bash
DOCKER_HOST=unix:///mnt/wsl/docker-desktop/shared-sockets/guest-services/docker.proxy.sock docker info
```

Observed result:

```text
permission denied while trying to connect to the docker API
```

The Docker Desktop user-distro proxy also cannot be started by the current
agent user because it needs to write `/run/docker-desktop-proxy.pid`.

Windows Docker fallback:

```bash
docker.exe info
```

Observed result: Docker Desktop was reachable and reported Server Version
`29.3.1`.

Using a temporary PATH shim so `scripts/smoke-docker-release.sh` resolved
`docker` to Windows `docker.exe`, the Docker build completed and produced:

```text
deve-notebook:local-smoke
```

The script then failed before endpoint verification with WSL interop instability:

```text
WSL (...) ERROR: UtilAcceptVsock: accept4 failed 110
```

After that failure, subsequent Windows executable calls from the agent process
returned:

```text
cannot execute binary file: Exec format error
```

Attempted required smoke command:

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 \
DEVE_DOCKER_SMOKE_PORT=3102 \
scripts/smoke-docker-release.sh
```

Assessment: REL-002 no longer blocks at image build when Windows Docker is used,
but the agent still cannot complete `docker run` plus `curl /api/node/role` from
a stable Linux Docker context. No application runtime endpoint failure has been
observed.

## Remaining Release Follow-Up

- Enable Docker Desktop WSL integration for this distro so plain Linux
  `docker info` works from the same shell that runs Codex, or run the smoke from
  a normal terminal with a stable Docker context.
- Re-run `DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_PORT=3102 scripts/smoke-docker-release.sh`.
- Keep `cargo test` / `cargo clippy --all-targets --all-features -- -D warnings`
  as the code gates before any release tag.

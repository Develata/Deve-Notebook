# Mainline Feature Implementation Selection - 2026-05-17

本报告记录从 platform credential / target-host handoff 返回后的主线功能实现选择。`docs/plan/` 未修改。

## Scope

- Current head before selection: `84d381a8`.
- Inputs: `docs/report/next-tasks.md`, latest platform evidence reports, `docs/features/`, `docs/acceptance-cases/`, current code.
- Goal: 选择下一批无需外部 signing material、target host、physical device 或 store credential 的主线功能实现批次。
- Non-goal: 打开 Desktop signing/notarization、Windows signed installer、Android signed APK/AAB、Android physical-device smoke、iOS signing/TestFlight/device smoke、native process runtime 或 native authority write gate。

## Guard Facts

- Latest current-head platform evidence has passed Docker Smoke and Native Target Host workflow on head `154fcc91`.
- Current platform post-gate scaffold is diagnostic/fail-closed by default.
- Latest mainline gap scan reports no blocking drift, no unbound acceptance case, and no new unblocked Current MUST.
- Platform post-gate continuation requires external target host or signing material. Those prerequisites are not available in this batch.

## Candidate Matrix

| Candidate | Evidence | External Dependency | Decision |
| --- | --- | --- | --- |
| Repo File Operations closure | `repo_file_operations.md`, `repo_file_op_shell_routing.md`, `STORE-012`, `STORE-013`, SearchBox file-op code, WS docs route, server docs handlers | None | Select |
| Settings local persistence / feedback | `settings_persistence_apply.md`, `settings_update.md`, `SET-003..006`; server-backed Settings API is explicitly outside current operation | None | Defer |
| Source Control command surface | `12_commands.md`, `CMD-004A..004C`; current Git mirror Web entries are CLI-only/unavailable notices | None | Defer |
| Platform signed / physical-device gates | release workflow preflight scaffold exists | Requires signing material, target host, or physical device | Blocked |

## Selected Batch

Next implementation batch: **Repo File Operations Closure**.

The batch should verify and close the user-visible create / rename / copy / move / delete flow through the existing modular path:

- SearchBox file-op shell parsing and candidate generation.
- Sidebar / Explorer operation surface where already wired.
- Local repo write gate and `scope_nonce` gate before sending.
- `ClientMessage::{CreateDoc,RenameDoc,CopyDoc,MoveDoc,DeleteDoc}` dispatch.
- CLI WS `route_docs` and server docs handlers.
- Projection refresh after structure mutation.

## Batch Rules

- Do not add a new authority path.
- Do not bypass current repo write gate or browser scope gate.
- Do not implement Web Git writer, Git repair executor UI, server-backed Settings API, platform signing, native process runtime, or native authority writes in this batch.
- Prefer narrow tests and one browser smoke report over broad refactor.

## First Implementation Targets

1. Re-run the existing repo file-op unit and server scope-gate tests to establish baseline.
2. Add or refresh a Chrome/browser smoke for create -> rename/move -> copy -> delete with reload/reconnect observation.
3. Fix only concrete gaps found in that smoke or in targeted tests.
4. Bind any new automation to the relevant acceptance case without changing `docs/plan/`.

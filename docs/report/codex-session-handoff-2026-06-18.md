# Codex Session Handoff

Date: 2026-06-18

## Scope

This report is a non-authoritative handoff snapshot for the long-running
Source Control, Git bridge, P2P FullPeer v1, native shell, and whole-codebase
risk review session.

No `docs/plan/` contract changed in this handoff batch. The next session must
continue to treat `docs/plan/` as authority and code as the projection of the
documented contract.

## Current State

- Repository: `E:\gitclone\Deve-Notebook`
- Branch: `main`
- Expected dirty state before the next task: only `.codegraph/.gitignore`,
  which was pre-existing unrelated state and must not be staged, edited, or
  reverted unless the user explicitly asks.
- Push policy: do not run `git push` without explicit user approval.
- User preference: continue in Chinese, make focused batch commits
  automatically after coherent verified batches.
- Active work is not complete. Do not mark the long-running review/fix goal as
  complete merely because this handoff exists.

## Required Workflow For Next Session

For any implementation or fix batch:

1. Read the nearest `AGENTS.md` files.
2. Read `docs/plan/00_engineering_constitution.md` and
   `docs/plan/01_terminology.md`.
3. Read the relevant `docs/plan/` chapter(s).
4. Read the matching `docs/features/` and `docs/acceptance-cases/` material
   when behavior or verification changes.
5. Modify docs first when the contract changes, then code/scripts.
6. Review the resulting diff for architecture and coupling risks.
7. Run targeted verification proportionate to the risk.
8. Commit the coherent batch, excluding unrelated dirty files.

The review standard remains risk-first: authority boundaries, repo/scope
fail-closed behavior, P2P identity/source attribution, low coupling, and
module boundary clarity matter more than broad line-by-line coverage.

## Recently Completed Commits

Recent relevant commits on `main`:

- `f20d4b40 align native release gate scripts`
- `7098b230 fix native acceptance commands`
- `32dc6565 harden desktop native service opt-in gate`
- `90d12b32 fix web source control remote read gate`
- `f747c7e5 harden p2p fullpeer framing tests`
- `2b22c7c6 test sync hello grant revocation`
- `9bae3c04 Reject third-party FullPeer request sources`
- `a22b5408 Validate config writer against runtime rules`
- `b1805b18 Fail closed anonymous localhost outside dev`
- `ceea68fe Type-gate delegated source control proxy`
- `9e00841f Reject non-canonical P2P peer ids`
- `7ed60b81 Track Git bridge mode in command palette`

## Key Completed Fixes

### Source Control And Git Bridge

- Browser HTTP Source Control mutations were separated from delegated remote
  proxy authority by typed authority paths.
- `REMOTE_PROXY_SCOPE_NONCE = 1` is no longer treated as ordinary browser HTTP
  mutation authority.
- Dev localhost anonymous behavior was tightened to fail closed outside dev.
- Config writer paths were aligned with runtime rules.
- Web command palette tracks Git bridge mode and Git writer actions remain
  CLI-only notices.
- A Web Source Control read regression was fixed: remote branch reads now use a
  read-only active-branch scope instead of requiring local writer readiness.

### P2P FullPeer

- Non-canonical P2P peer ids are rejected.
- Third-party FullPeer request sources are rejected for the current v1 bridge,
  avoiding unsupported source attribution until source proof exists.
- P2P FullPeer framing tests now fail closed on text server frames.
- The accepted architecture direction remains: do not share one private key
  across peers. Use per-peer identity keys, repo-level membership/trust, and
  source proofs for any future multi-hop third-party shadow forwarding.

### Native Shell And Release Gates

- Desktop local service entrypoint now requires both `DEVE_NATIVE_AUTHORITY=1`
  and `DEVE_DESKTOP_LOCAL_SERVICE=1`; invalid values fail closed.
- Acceptance docs no longer mention nonexistent Cargo features
  `native_authority` or `embedded_service`.
- Native and release baseline scripts now check current module/function names
  instead of stale paths.

## Verification Already Run

Source Control / Web:

- `bash scripts/check-source-control-baseline.sh`
- `cargo test -p deve_web source_control_remote_read -- --nocapture`
- `cargo test -p deve_web source_control_read -- --nocapture`
- Targeted command palette and Source Control tests around Git bridge mode.

P2P / Network:

- `bash scripts/check-network-baseline.sh`
- Targeted core/CLI P2P and sync tests around hello, snapshot, p2p mesh,
  canonical peer ids, and grant revocation.

Native / Release:

- `cargo test -p deve_desktop --features native-packaging service_entrypoint -- --nocapture`
- `cargo test -p deve_core native_adapter -- --nocapture`
- `cargo test --locked -p deve_core --lib native_adapter::process_test -- --nocapture`
- `cargo test --locked -p deve_desktop desktop_default_build_defers_real_process_adapter -- --nocapture`
- `cargo test --locked -p deve_mobile mobile_default_build_defers_real_process_adapter -- --nocapture`
- `cargo test --locked -p deve_cli native_session -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging service_bootstrap -- --nocapture`
- `cargo test --locked -p deve_desktop process_observation -- --nocapture`
- `cargo test --locked -p deve_mobile process_observation -- --nocapture`
- `cargo test -p deve_desktop --features native-packaging -- --nocapture`
- `cargo test -p deve_mobile --features native-packaging -- --nocapture`
- `./scripts/check-release-baseline.sh`
- `./scripts/check-native-track-boundary.sh`
- `./scripts/check-native-process-adapter-gate.sh`
- `cargo fmt --check`
- `git diff --check`

## Environment Notes

- Windows PowerShell is the current shell.
- The system `bash`/WSL shim failed in this session with
  `Wsl/Service/CreateInstance/E_ACCESSDENIED`.
- Scoop Git Bash worked when run with elevated tool execution:
  `C:\Users\QQ\scoop\apps\git\2.54.0\bin\bash.exe`.
- If a script fails with a WSL access-denied error, retry the same script via
  Scoop Git Bash before treating it as a project failure.
- The user mentioned a WSL clone at `~/gitclone/Deve-Notebook/`, but WSL was
  not reachable in this session. Re-check before using it.

## Suggested Next Review Pass

Recommended continuation order:

1. Start with `git status --short` and confirm only `.codegraph/.gitignore`
   is dirty.
2. Re-read the plan chapters for the module under review, especially:
   - `docs/plan/02_architecture.md`
   - `docs/plan/05_diff_logic.md`
   - `docs/plan/07_network.md`
   - `docs/plan/12_tech_release.md`
   - `docs/plan/14_commands.md`
   - `docs/plan/15_settings.md`
   - `docs/plan/19_plugins.md`
3. Continue module review after the native script batch. High-value next
   targets:
   - `apps/cli/src/server/runtime/` and node role/reporting paths.
   - `apps/web/src/api/connection_role.rs` and related runtime readiness UI.
   - P2P source attribution and future multi-hop design boundaries.
   - Any remaining Source Control/Git bridge paths that can write authority or
     enqueue Git bridge work.
4. Small, isolated bugs may be fixed directly after document/code inspection.
   For large protocol, authority, or identity changes, stop and present the
   architecture tradeoff before editing.

## Carry-Forward Architecture Constraints

- Deve ledger / `.notegit` remains the only Source Control authority.
- Native Git is only an explicit mirror/import/export/push bridge.
- `source_control.git_bridge = "off"` must keep Deve Source Control usable but
  block Git bridge writes except read-only status diagnostics.
- Browser Source Control writes must be tied to the current browser session
  grant, not a global dev-wide permission.
- Delegated remote proxy authority must stay a distinct typed path.
- P2P v1 must remain static FullPeer; no automatic discovery, NAT traversal,
  relay marketplace, automatic merge, or `WS_PROTOCOL_VERSION` bump unless the
  plan is explicitly reopened.
- Multi-hop third-party shadow forwarding requires source proofs and repo trust
  membership before it can be accepted.
- Native shells remain no-authority/no-packaging by default; local service
  authority is explicit opt-in only.

## New Session Prompt

Use this prompt to restart in a fresh goal-mode session:

```text
你是 Codex，在 E:\gitclone\Deve-Notebook 中继续上一轮长期审查与修复工作。请始终使用中文。当前目标不是从零开始，而是接着做“全体代码风险优先审查 -> 小批修复 -> review -> 验收 -> commit”的连续工作。

先执行非破坏性摸底：读取根 AGENTS.md、docs/AGENTS.md、相关子目录 AGENTS.md；读取 docs/report/codex-session-handoff-2026-06-18.md；运行 git status --short 和 git log --oneline -12。预期分支是 main，预期唯一无关脏文件是 .codegraph/.gitignore。不要修改、stage 或 revert 这个文件，除非我明确要求。不要 git push，除非我明确同意。

必须遵守项目工作流：任何实现或修复前，先读 docs/plan/00_engineering_constitution.md 与 docs/plan/01_terminology.md，再读相关 docs/plan 章节，然后读匹配的 docs/features 与 docs/acceptance-cases，最后再改代码或脚本。docs/plan 是权威合同，代码只是投影。遇到代码与 plan 不一致时，默认按实现漂移处理，除非有明确 registry/ADR 证据。

继续沿用上一轮的架构约束：Deve ledger / .notegit 是唯一 Source Control authority；原生 Git 只作为显式 mirror/import/export/push bridge；source_control.git_bridge=off 必须保留 Deve Source Control 可用但阻止 Git bridge 写入；Browser Source Control 写必须绑定当前 browser session grant，不允许回到全局 dev-wide grant；Delegated remote proxy 必须保持单独 typed authority path；P2P v1 只稳定静态 FullPeer，不做自动发现、NAT、公共 relay marketplace、自动 merge，也不 bump WS_PROTOCOL_VERSION；不要采用多个 peer 共用一个私钥的方案，未来多跳第三方 shadow 转发应使用每 peer 独立身份 key、repo trust membership、payload source proof；native shell 默认无 authority、无 packaging，local service authority 必须显式 opt-in。

本轮已经完成并提交过的关键批次包括：Source Control/Git bridge gate 与 delegated path 分离、dev localhost anonymous fail-closed、config writer runtime validation、P2P canonical peer id 与第三方 source reject、P2P framing tests、Web Source Control remote read gate 修复、Desktop native local service 双 env opt-in gate、native acceptance command 修正、release/native baseline scripts 对齐。最近提交可通过 git log 查看，交接文档中有列表。

继续 review 的建议顺序：先检查 apps/cli/src/server/runtime/ 和 node role/reporting 路径，再检查 apps/web/src/api/connection_role.rs 与 runtime readiness UI，然后回到 P2P source attribution/multi-hop 设计边界，最后扫剩余 Source Control/Git bridge 写入口。审查标准按风险优先：authority 越界、repo/scope fail-closed、writer grant、P2P identity/source attribution、watcher/writeback 环路、全局状态/锁、错误类型边界、路径规范化、模块低耦合度。

执行方式：如果发现小而明确的 bug，可以按 docs->code 顺序直接修复、review、运行针对性测试并 commit。若发现会改变协议、identity、authority、repo trust、source proof 或大范围模块边界的问题，先停止并向我说明方案和权衡，不要贸然改。每个 coherent batch 完成后自动 git add 精确文件并 git commit，不要把无关脏文件带入 commit。

验证要求：每个修复批次必须运行与风险匹配的 cargo test / baseline 脚本 / cargo fmt --check / git diff --check。若 bash 或 WSL 因 Wsl/Service/CreateInstance/E_ACCESSDENIED 失败，优先用 C:\Users\QQ\scoop\apps\git\2.54.0\bin\bash.exe 重跑脚本；必要时申请 elevated execution。最终回复必须写“验证：...”或“未运行：...及原因”。
```

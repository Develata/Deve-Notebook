# Merge Conflict UI Browser Smoke - 2026-05-13

本报告记录 `MergePeer` 冲突 UI 的真实浏览器点验。`docs/plan/` 仍是唯一权威；本文件只记录实机证据与本批修复。

## Scope

- 唯一真源：`docs/plan/07_diff_logic.md#merge-contract`
- 验收目标：验证 `MergeConflict` UI 可见、三个 resolve action 可执行，并携带稳定 `doc_id` / `action` / `result_content` / `scope_nonce`。
- 覆盖链路：hidden conflict fixture -> `serve --dev` -> remote peer branch -> command palette `P2P: 合并当前节点` -> conflict UI -> `ResolveMergeConflict` -> local projection。

## Fixture

- 新增 hidden CLI：`deve_cli seed-merge-conflict-fixture`
- Fixture 行为：
  - 创建本地 repo 文档结构。
  - 创建同一 repo 的 remote shadow 结构。
  - 写入 shared base。
  - 本地写入 `local`。
  - remote shadow 写入 `remote`。
  - 将本地 vault projection 固定为 `local`。

## Findings

- 首轮 browser smoke 暴露 command palette action 在点击时读取 Leptos context 并 panic。修复为命令注册时捕获 `BranchContext` / `SyncMergeContext` / `CoreState`，点击时 fail-soft。
- 首轮 browser smoke 暴露 `MergePeer` 被普通 local write gate 阻断。修复为专用 peer-branch scope gate：当前 repo、active branch 等于 peer、且无 pending repo/branch switch 时允许显式 merge。
- 首轮 browser smoke 暴露 server 以 local branch scope 发送冲突消息，browser 在 remote branch 下拒收。修复为 path authority 使用 local scope、message scope 使用当前 remote branch scope。
- 首轮 browser smoke 暴露 typed `MergeConflict` 被 legacy `DocDiff` fallback 覆盖。修复为当前 diff 已是同一 doc/path 的 merge conflict 时忽略未请求的 fallback `DocDiff`。
- 首轮 browser smoke 暴露 `accept-both` 默认发送 incoming-only 内容。修复为非编辑态发送确定性 `current + incoming`，编辑态继续发送用户编辑内容。

## Browser Smoke

- Web assets：`scripts/smoke-web-release-build.sh`
- Browser tool：Chrome MCP
- Server mode：`serve --dev`
- Fixture command：`seed-merge-conflict-fixture --peer peer-a`
- 操作序列：
  - 打开本地 `notes/conflict.md`。
  - 切换到 `peer-a` branch。
  - 打开 remote `notes/conflict.md`。
  - command palette 执行 `P2P: 合并当前节点`。
  - 验证 conflict UI 出现 `accept-current` / `accept-incoming` / `accept-both`。

## Result

- `accept-current`：
  - 数据根：`/tmp/deve-merge-conflict-ui-current-20260513182157`
  - vault 文件：`vault/default/notes/conflict.md`
  - 结果内容：`local`
- `accept-incoming`：
  - 数据根：`/tmp/deve-merge-conflict-ui-incoming-20260513182305`
  - vault 文件：`vault/default/notes/conflict.md`
  - 结果内容：`remote`
- `accept-both`：
  - 数据根：`/tmp/deve-merge-conflict-ui-20260513181959`
  - vault 文件：`vault/default/notes/conflict.md`
  - 结果内容：
    ```md
    local
    remote
    ```
- `ProtocolError` banner 中的 `Merge Conflict detected. Showing Diff View.` 是结构化冲突通知，不是 runtime failure。
- 停止 server 后浏览器出现的 WS connection refused 属于测试收尾阶段的预期断线。

## Automated Verification

已运行：

- `cargo test -p deve_cli merge_conflict_fixture -- --nocapture`
- `cargo test -p deve_cli merge_peer -- --nocapture`
- `cargo test -p deve_cli resolve_merge_conflict -- --nocapture`
- `cargo test -p deve_web merge -- --nocapture`
- `cargo test -p deve_web accept_both -- --nocapture`
- `cargo test -p deve_web stable_peer_branch_scope -- --nocapture`
- `cargo test -p deve_web requested_doc_diff -- --nocapture`
- `cargo fmt --check`
- `git diff --check`
- `scripts/plan-coverage.sh`
- `scripts/check-architecture-registry.sh`

结果：全部通过。

## Status

`Merge conflict UI browser smoke` 可关闭。后续 Source Control merge 工作应继续使用该 hidden fixture 作为 deterministic browser seed，不依赖 checked-in runtime ledger/vault。

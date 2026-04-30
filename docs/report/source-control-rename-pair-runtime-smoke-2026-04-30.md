# Source Control Rename Pair Runtime Smoke - 2026-04-30

## Scope

验证 active vault 中已由 Ledger 管理的 Markdown 文件被外部 rename 后，watcher / Source Control HTTP / Web UI 的当前行为。

## Environment

- Temp root: `/tmp/deve-rename-smoke-fixed.Rgqqvc`
- Server: `DEVE_LEDGER_DIR=.../ledger DEVE_VAULT_PATH=.../vault cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3104`
- Browser: Chrome MCP at `http://127.0.0.1:3104/`
- Repo: `default`

## Result

- Web UI 创建 `Untitled.md` 后，`GET /api/sc/status` 返回空数组，baseline clean。
- 外部执行 `Untitled.md -> Renamed-runtime.md` 后，Source Control Web 面板显示一条 rename 行：`Untitled.md -> Renamed-runtime.md`，状态标记为 `R`。
- `GET /api/sc/status` 返回前端展示用的折叠 rename row：
  - `path = Renamed-runtime.md`
  - `renamed_from = Untitled.md`
  - `doc_id = d3ec9f45-5a0d-4bee-ae84-b64c12198056`
  - `status = Added`
  - `has_conflict = true`
- 内部 `pending_fs_ops` rename 表达仍由代码级测试守住 delete/add pair；HTTP read service 当前会通过 `collapse_rename_candidates` 折叠掉 deleted side，只暴露一条 rename row 给 Web。
- 页面无 rate-limit / 频率限制文案。
- Chrome console 无 warn/error。
- 修复后服务端在 rename 后继续观察约 10 秒，没有再出现重复 `Handler: Rename detected ...` 日志刷屏。

## Code Follow-Up Closed

初次 smoke 暴露出重复语义 rename 事件会在 handler 层先记录 `Rename detected`，再由 pending upsert 判定 no-op，导致运行时日志看起来像循环。

本批次将 `Handler: Rename detected` 日志移动到 `pending_rename::upsert_external_rename` 真实写入之后；重复语义事件不再产生刷新消息，也不再产生日志刷屏。

新增回归测试覆盖 plain file event 路径：

```bash
cargo test -p deve_core sync::watcher::dispatch_test::dispatch_batch_suppresses_duplicate_rename_refresh_from_plain_events -- --nocapture
```

## Verification

- `cargo fmt --check`
- `cargo test -p deve_core sync::watcher::dispatch_test::dispatch_batch_suppresses_duplicate_rename_refresh_from_plain_events -- --nocapture`
- Chrome MCP runtime smoke as above


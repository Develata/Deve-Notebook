# Git Mirror Repair Review Web Consumption - 2026-04-29

## 已完成

- Web 新增 `fetch_git_mirror_repair_review` 只读 API client，访问
  `GET /api/sc/git-mirror/repair-review`。
- Source Control repair notice 在可读且当前 notice 为 Git repair 时拉取 record-level
  review data。
- UI 优先展示 server-side action code、subject、next step、retry command 与 authority note；
  endpoint 失败、无 record 或 notice 变化时回退原 CLI-only 静态 review。

## 边界

- Web 仍不执行 Git，不调用 `deve_cli git export`，不写 `.git` / `.notegit`。
- Web 不解析 CLI output；record-level 数据只来自 protected HTTP read endpoint。
- 当前 UI 仍是 read-only review，不提供 manual confirmation 或 executable repair。

## 已验证

- `cargo test -p deve_cli test_git_mirror_repair_review_is_readonly_record_source -- --nocapture`
- `cargo test -p deve_web local_git_repair_notice -- --nocapture`
- `cargo test -p deve_web repair_review_url_uses_repo_id_when_available -- --nocapture`

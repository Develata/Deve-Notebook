# Git Mirror Future Boundary Audit - 2026-04-30

## 结论

当前 Git mirror 能力边界清晰：写 Git 只允许通过显式 CLI surface 触发；Web 只允许展示 CLI-only notice 与只读 repair review。后台 Git writer、Web 后端直接执行 Git import/push/repair、executable Web repair UI 仍保持 future/deferred，不属于当前 active queue。

## 已确认当前能力

- CLI 已提供 `deve_cli git mirror`、`deve_cli git export`、`deve_cli git import --apply` 与 `deve_cli git push`。
- Web Command Palette 已提供 `Git: Import Changes`、`Git: Push Mirror`、`Git: Repair Mirror` 的可发现入口，但只显示 CLI-only notice。
- Web repair review 数据源是 `GET /api/sc/git-mirror/repair-review` 只读 endpoint，只读取 server-side mirror record 与 repair-action schema。
- Source Control repair notice 可消费只读 review，展示多条 out-of-sync record、loading、failed 与 empty fallback 状态。

## 明确不属于当前能力

- 不实现后台自动 Git mirror executor。
- 不实现 Web 后端直接执行 Git import/push/repair。
- 不实现 executable Web repair UI。
- 不把 repair review endpoint、Command Palette notice、CLI import apply 或 CLI push surface 解释为 Web Git writer。

## 本批次同步

- `docs/report/git-mirror-bridge-status-2026-04-29.md` 将“仍未实现”改为 future/deferred 语义，避免被误读为 active queue。
- `scripts/check-source-control-baseline.sh` 增加 Git mirror future/partial 边界守卫，固定 Web read-only / CLI-only、future executable repair UI 与无后台 executor 语义。

## 验证

- `scripts/check-source-control-baseline.sh`
- `git diff --check`

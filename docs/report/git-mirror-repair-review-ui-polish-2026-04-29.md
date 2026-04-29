# Git Mirror Repair Review UI Polish - 2026-04-29

## 已完成

- Source Control repair review copy 已拆到独立 `repair_review_copy` 模块，避免通用 error notice 继续膨胀。
- Review UI 支持多条 out-of-sync record 展示，每条 record 独立显示 action、commit、subject、next step、manual-only guidance 与 retry command。
- Fetch 状态从隐式 `Option` 升级为 `Idle / Loading / Loaded / Failed`，UI 显示 loading、load failed、empty record fallback。

## 边界

- UI 仍只读，不执行 Git，不调用 clipboard API，不解析 CLI 输出。
- Endpoint 失败或无 record 时只回退 CLI-only 静态指引，不触发任何 repair writer。
- `.notegit` / ledger source-control state 仍是 authority，`.git` 仍只是 projection mirror。

## 已验证

- `cargo test -p deve_web repair_review -- --nocapture`
- `cargo check -p deve_web --all-targets`

# Git Mirror Read-only Repair Review Status - 2026-04-29

## 已完成

- Source Control repair notice 新增只读 repair review scaffold，使用 `data-deve-git-repair-review="readonly"` 标记。
- Review scaffold 展示 repair action、guidance、subject、next step、retry command 与 `.notegit` authority note。
- Retry command 只以可选择的 monospace 文本展示，并带 `data-deve-git-repair-retry-command`；没有 clipboard API、没有 Web 后端调用、没有 Git writer。
- `data-deve-git-repair-manual-only="true"` 明确当前 UI 只读，符合 manual-only repair boundary。

## 工程边界

- 本批次只消费本地 `SourceControlNotice::git_repair_cli_only()` 与 i18n 文案。
- Web 仍不读取 `.git` / `.notegit`，不解析 CLI 输出，不调用 `deve_cli git export`。
- `.notegit` / ledger source-control state 仍是 authority；`.git` 仍只是 projection mirror。

## 后续

- 若要让 review 展示真实 record-level `repair_action[...]`，下一步应先决定只读数据来源：HTTP status endpoint、server-side status query，或显式 CLI copy/paste，不得直接引入 Git writer。
- Confirmation UI 只能在只读数据来源和 fail-closed gate 明确后再做。

## 验证

- `cargo test -p deve_web local_git_repair_notice_uses_cli_copy -- --nocapture`
- `cargo test -p deve_web git_bridge_source_control_copy_is_localized -- --nocapture`
- `cargo check -p deve_web --all-targets`
- `scripts/plan-coverage.sh`
- `git diff --check`

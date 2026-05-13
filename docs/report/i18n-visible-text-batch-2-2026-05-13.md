# I18N Visible Text Batch 2

日期：2026-05-13

## 范围

- 计划锚点：`docs/plan/11_i18n.md#i18n-facade-contract`
- 验收锚点：`docs/acceptance-cases/09_i18n.md#I18N-001`
- 代码范围：Web shell controls、sidebar item actions、disconnect overlay。

## 结果

- ActivityBar more action title 改为 `t::sidebar::more_actions`。
- Sidebar item action title 改为 `t::sidebar::more` / `t::common::new_file`。
- Disconnect overlay status line 改为 `t::common::status` + localized status copy。
- `scripts/check-i18n-hardcoded-baseline.sh` 扩展到本批 shell/sidebar/disconnect 范围。

## 非目标

- 产品名、协议常量、CSS class、DOM marker 与测试 fixtures 不属于本批。
- Source Control 深层错误说明、Graph/Settings/AI 已有独立 i18n 模块，不在本批重复迁移。

## 验证

- `bash scripts/check-i18n-hardcoded-baseline.sh`
- `cargo test -p deve_web disconnected_lockdown -- --nocapture`
- `cargo fmt --check`

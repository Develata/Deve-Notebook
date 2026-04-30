# Web Release Browserslist Warning Triage - 2026-04-30

## Scope

调查 `trunk build --release` 中的 `Browserslist: caniuse-lite is outdated` 提示是否属于 `apps/web/package-lock.json` 可直接更新的问题。

## Findings

- `apps/web/package-lock.json` 不包含 `browserslist`、`caniuse-lite`、`update-browserslist-db`、`tailwindcss`、`postcss` 或 `autoprefixer` 锁定条目。
- `apps/web/node_modules` 当前也没有这些包的本地安装。
- 提示来自 `index.html` 中 `data-trunk rel="tailwind-css"` 触发的 Trunk Tailwind pipeline，而不是 Web npm lockfile 中的可更新依赖。
- 因此，本批次不运行 `npx update-browserslist-db@latest`，也不修改 Web npm dependency graph。

## Change

- 新增 `scripts/smoke-web-release-build.sh`，统一本地 Web release asset build：
  - 把 `NO_COLOR=1` 归一化为 Trunk 0.21 可接受的 `NO_COLOR=true`。
  - 设置 `BROWSERSLIST_IGNORE_OLD_DATA=true`，抑制非 lockfile 可修复的 Browserslist DB freshness 提示。
- `docs/dev-runbook.md` 改用该 wrapper 作为 embedded frontend 推荐构建入口。
- Docker frontend stage 使用相同环境运行 `trunk build --release`，保持 release build 日志稳定。
- `check-dev-runbook-baseline.sh` 与 `check-release-baseline.sh` 更新对应守卫。

## Verification

- `scripts/smoke-web-release-build.sh`
- `scripts/check-dev-runbook-baseline.sh`

## Decision

当前 `caniuse-lite is outdated` 不作为产品代码或 lockfile drift blocker。若未来 `apps/web/package-lock.json` 引入 `browserslist/caniuse-lite`，再重新打开依赖更新批次。


# Acceptance / Release Guard Cleanup

日期：2026-05-14

## 结论

Acceptance parser、CLI command baseline 与 `REL-001` release surface 已对齐当前代码与 release workflow。

## 变更

- `scripts/check-acceptance-bindings.sh` 的 `case_id` 提取支持 `CMD-007A` / `CMD-007B` 这类数字后缀字母 ID，不再截断为 `CMD-007`。
- `docs/acceptance-cases/11_commands_settings.md` 与 `scripts/check-cli-settings-baseline.sh` 补齐 Graph、Source Control status、Git bridge 子命令、node-check 等当前 CLI surface。
- `REL-001` 不再检查过时的 `dist/v1.0.0` 目录产物，改为检查 tag-triggered release workflow、semver/latest Docker metadata 与 GHCR image surface。
- Release operation docs 去掉 `ci-or-dist` / `dist/` 作为当前 release channel surface，收敛为 workflow 与 registry metadata。

## 验证

- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-cli-settings-baseline.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`

## 后续

- 当前 active queue 中只剩需要明确许可的 plan patch 与 native packaging gate opening decision；默认继续回到 mainline implementation gap scan。

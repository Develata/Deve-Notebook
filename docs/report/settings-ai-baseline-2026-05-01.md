# Settings / AI Baseline - 2026-05-01

本报告合并 Search/settings boundary、AI effective config、Native AI Chat、AgentBridge env alias、browser prefs 与 Settings UI acceptance-depth 报告。

## Current Boundary

- Search 当前为 `search` feature + non-low-spec 下的 repo-scoped baseline scan；Tantivy 常驻索引仍是 future optimization。
- Settings 当前以 `config.toml` 与 `deve config print/set` 为稳定入口；server-backed Settings API 与统一 GUI 持久化仍是 future。
- `ai.mode` / `ai.native_enabled` 驱动 server provider/RPC、Trusted CLI policy、capabilities endpoint 与 Web fallback/disabled UI。
- Native AI Chat 只提供最小 PLAN/BUILD、bounded history、受控 BUILD Apply；默认不暴露 shell/MCP/tool loop。
- `trusted-cli` default-off；`DEVE_AI_AGENT_BRIDGE_ENABLED` / `DEVE_AI_AGENT_BRIDGE_TRUSTED` 仅作为 Trusted CLI policy 兼容输入。
- harmless browser UI prefs 统一走非权威 fallback storage；业务 authority 不得落入 browser storage。

## Verified Surfaces

- AI backend capability policy、fallback/disabled reason、provider registration gate。
- Settings sync mode、language、reserved/future control、AI backend button policy helpers 与代码级测试。
- Browser UI prefs fallback 层：layout width、Outline visibility、locale preference、shortcut overrides。
- `scripts/check-cli-settings-baseline.sh`。

## Retired Source Reports

- `agent-bridge-env-alias-plan-sync-2026-04-30.md`
- `ai-effective-config-boundary-status-2026-04-30.md`
- `ai-settings-ui-acceptance-depth-2026-04-30.md`
- `browser-ui-prefs-boundary-status-2026-04-30.md`
- `native-ai-chat-boundary-audit-2026-04-30.md`
- `search-settings-boundary-audit-2026-04-29.md`
- `settings-language-ui-acceptance-depth-2026-04-30.md`
- `settings-reserved-ui-acceptance-depth-2026-04-30.md`
- `settings-sync-ui-acceptance-depth-2026-04-30.md`

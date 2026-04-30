# Core Hardening Baseline - 2026-05-01

本报告合并 P0/P1 hardening、rendering boundary、architecture registry sync 与 post-verification drift rescan 的短报告。

## Current Boundary

- Sync vector wire contract 使用 `DEVEWSF3` 与显式 `known_vector/server_vector`。
- Browser storage degraded mode 必须 read-only / write-gated。
- `identity.key` owner-only、login audit metadata、CORS wildcard fail-closed、dev-only auth/CORS warnings 已作为安全硬化基线。
- Runtime path normalization 集中到 `deve_core::utils::path::to_forward_slash`。
- Rendering 当前区分 main editor adapter、lightweight Markdown renderer 与 future preview/virtual-render/settings GUI。
- Architecture registry operation IDs 必须随 operation specs 同步。

## Verified Surfaces

- Sync/storage/auth/path normalization targeted tests。
- Rendering current/future boundary guard。
- Architecture registry check。
- Post-verification plan/code drift rescan。
- Rendering guard 保留主编辑器视觉投影、聊天/辅助 HTML 轻量 Markdown 渲染、大文档批量调度三条边界。
- Chat BUILD Apply 仍由 BUILD mode 门禁控制，并通过带当前 scope nonce 的 `ClientMessage::Edit` 写入。
- Lightweight renderer 测试覆盖 code-block apply affordance、`<br>` allowlist、安全外链、unsafe scheme 降级，以及 unsupported highlight 保持普通文本。
- Architecture registry sync 保留 `op.i18n.locale.fallback-unsupported`、Native AI Chat operation IDs 与 release Web WASM quality gate operation。

## Retired Source Reports

- `architecture-registry-operation-id-sync-2026-04-30.md`
- `p0-sync-storage-status-2026-04-28.md`
- `p1-path-normalization-status-2026-04-28.md`
- `p1-security-hardening-status-2026-04-28.md`
- `post-p2-plan-code-drift-rescan-2026-04-30.md`
- `post-verification-plan-code-drift-rescan-2026-04-30.md`
- `rendering-current-boundary-baseline-2026-04-30.md`

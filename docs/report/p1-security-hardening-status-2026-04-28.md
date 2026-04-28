# P1 Security Hardening 状态 - 2026-04-28

## 结果

- Security hardening small batch 已完成。
- 本批次只收口 `docs/plan/09_auth.md` 中会影响启动安全、登录审计与跨站来源边界的当前能力；未扩展到完整 TLS/CSRF future 项。

## 已实现

- `identity.key` 加载/生成后会校正为 owner-only 权限。
  - Unix 平台强制设置为 `0600`。
  - 非 Unix 平台检查路径必须是普通文件，并显式记录权限不可移植 warning。
- 登录成功/失败审计改为结构化 `Login audit`。
  - 字段包含 `success`、`user`、`ip`、`timestamp`、`user_agent`。
  - 缺失或空白 `User-Agent` 归一化为 `unknown`。
- CORS origin 解析改为显式 fail-closed。
  - `ALLOWED_ORIGINS=*` 直接报错。
  - 仍要求显式 origin 带 scheme 与 authority。
- dev-only 行为的 warning 文案更明确。
  - development auth defaults 标记为 `development-only`。
  - anonymous localhost auth bypass 首次触发时显式 warning。
  - development CORS allow list 标记为 development-only。

## 验证

- `cargo fmt`
- `cargo fmt --check`
- `git diff --check`
- `scripts/check-auth-baseline.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `cargo test -p deve_cli auth -- --nocapture`
- `cargo test -p deve_cli setup -- --nocapture`
- `cargo test -p deve_cli security -- --nocapture`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`

## 后续边界

- `AUTH_SECRET` 本身来自环境变量；当前批次未引入独立 secret file，因此不存在可校正的 auth secret file path。
- `ALLOWED_ORIGINS` 未设置时保持空 allow-list，不自动放开任何跨站 origin；生产部署若需要跨站访问，必须显式配置白名单。
- CSRF 的额外 Origin/Referer 校验仍可作为后续 hardening 项，但当前主防线仍是 `SameSite=Strict` 与 authenticated write boundary。

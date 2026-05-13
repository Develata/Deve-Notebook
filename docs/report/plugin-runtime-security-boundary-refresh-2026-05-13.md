# Plugin Runtime Security Boundary Refresh - 2026-05-13

本报告记录 `PLUG-003`、`AI-005`、`AI-006` 相关边界复核与本批修复。`docs/plan/` 仍是唯一权威；本文件只记录当前实现事实。

## Scope

- `docs/plan/17_plugins.md`
- `docs/plan/10_ai_agent.md`
- `docs/acceptance-cases/10_plugins.md`
- `crates/core/src/plugin/`
- `apps/cli/src/server/agent_bridge/`
- `apps/web/src/api/ai_backend.rs`
- `apps/web/src/components/chat/actions/send_backend.rs`

## Findings

Confirmed:

- Manifest entry path already rejected absolute paths, parent traversal, backslashes, non-`.rhai` suffixes, and symlink escape.
- Host FS path guard already canonicalized symlink targets before capability matching.
- `fs_write` already rejected ledger-managed Markdown and `.notegit` / Git mirror internal paths.
- Rhai `eval` was already disabled.
- `env()` already failed closed for undeclared variables and returned Unit only for declared-but-unset variables.
- Trusted CLI policy already failed closed unless enabled, trusted, absolute, existing, and executable.
- Trusted CLI send path already fell back to Native with visible reason when policy rejected it.

Fixed:

- Rhai import resolution now uses `GuardedFileModuleResolver` instead of a bare `FileModuleResolver`.
- Imported module paths must be relative forward-slash paths with normal segments and optional `.rhai` suffix.
- Imported modules are canonicalized before load and must remain under the plugin directory.
- Symlinked imported modules that resolve outside the plugin directory are rejected.
- `PLUG-003` now has an automated run binding for the module resolver guard.
- `check-ai-baseline.sh` now guards the module resolver and its traversal / symlink tests.

## Verification

Ran:

- `bash scripts/check-ai-baseline.sh`
- `cargo test -p deve_core plugin::runtime::module_resolver -- --nocapture`
- `cargo test -p deve_core plugin::runtime::rhai_v1 -- --nocapture`
- `cargo test -p deve_core plugin -- --nocapture`
- `cargo test -p deve_cli agent_bridge -- --nocapture`
- `cargo test -p deve_web trusted_cli_default_off -- --nocapture`
- `cargo test -p deve_web trusted_cli_untrusted -- --nocapture`
- `cargo test -p deve_web chat_send -- --nocapture`

Results:

- AI baseline: pass.
- Core plugin boundary tests: pass.
- Agent bridge policy and HTTP capability tests: pass.
- Web trusted-cli default-off / fallback send tests: pass.

## Decision

Plugin runtime security boundary refresh is closed. Continue with Protocol error / version alignment capture.

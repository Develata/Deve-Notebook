# Command Surface Reserved Boundary - 2026-05-16

本报告记录 Post-regression Implementation Selection 后的 Command Palette 小批次。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/12_commands.md`.
- Close the remaining command-surface gap from `mainline-gap-scan-after-native-target-host-closure-2026-05-16.md`.
- Do not add Web Git writer, background Git executor, native process runtime, or native authority writes.

## Changes

- Added Command Palette entries for `Git: Status`, `Git: Mirror`, and `Git: Export Mirror`.
- These Git entries are unavailable/CLI-only in the UI and only surface Source Control notices that point users to CLI commands.
- Added unavailable Command Palette entries for unbound Source Control command surfaces: sync, commit, and push.
- Added unavailable Command Palette entries for unbound AI command surfaces: retry, switch backend, switch PLAN, and switch BUILD.
- Added focused acceptance cases `CMD-004B` and `CMD-004C`.
- Extended CLI/settings and Source Control baseline guards.
- Split new command tests into dedicated test modules to avoid adding file-size soft warnings.

## Verification

Ran:

- `cargo fmt --check`
- `cargo test -p deve_web command_palette -- --nocapture`
- `cargo test -p deve_web source_control_notice -- --nocapture`
- `cargo test -p deve_web source_control_copy_is_localized -- --nocapture`
- `scripts/check-cli-settings-baseline.sh`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/plan-coverage.sh`

Results:

- Command Palette tests: `12` passed.
- Source Control notice tests: `5` passed.
- Source Control i18n copy test: `1` passed.
- CLI/settings baseline: pass.
- Source Control baseline: pass.
- Acceptance bindings: `108` automated, `60` feature walkthrough, `29` manual, `0` unbound soft cases.
- Plan coverage: `0` blocking violations, `17` existing soft file-size warnings.

## Decision

Command-surface G6 coverage is closed for the current boundary.

The Web UI now distinguishes:

- executable command entries with an implemented backend contract;
- CLI-only Git mirror entries that cannot execute Web Git writer paths;
- unavailable Source Control / AI entries that remain visible without claiming unimplemented capability.

Next executable work should rescan the mainline from this green boundary before opening another implementation batch.

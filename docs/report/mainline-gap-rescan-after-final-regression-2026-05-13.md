# Mainline Gap Rescan After Final Regression - 2026-05-13

本报告记录 `final-regression-gate-2026-05-13.md` 后的新一轮 gap scan 与首个小批次修复。`docs/plan/` 仍是唯一权威；本文件只记录当前实现事实。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/features/operations/`, `docs/acceptance-cases/`, `docs/acceptance-bindings.tsv`, current code and guard scripts.
- Non-goal: reopen native packaging gate, server-backed Settings API, Graph renderer, Calculation Runtime, plugin marketplace, or MCP runtime.

## Verification Snapshot

Ran:

- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-native-packaging-gate.sh`

Results:

- Acceptance binding: `94 automated / 61 feature / 29 manual / 0 unbound`.
- Architecture registry: `72 flows, 0 active drift`.
- Feature operation paths: pass.
- Native packaging gate: pass; Desktop/Mobile remain no-packaging skeletons.

## Selected Gap

`11_i18n.md#i18n-facade-contract` requires core UI text to flow through `t::*`.

The scan found Command/Search visible helper text still embedded in component/provider code:

- Command Palette and Unified Search keyboard footer copy.
- Search command result detail.
- Branch result detail.
- File create/open result copy.
- File operation result titles, details, group labels and local parse errors.

This is a Current MUST gap, but the correct batch size is limited to Command/Search. A full frontend hardcoded-text rewrite would mix unrelated UI surfaces and increase regression risk.

## Fixes

- Added localized Command/Search helper strings under `apps/web/src/i18n/command_palette.rs` and `apps/web/src/i18n/search.rs`.
- Passed `Locale` into Command, Branch, File and FileOp search providers.
- Replaced targeted visible hardcoded strings with `t::*` facade calls.
- Added `scripts/check-i18n-hardcoded-baseline.sh` as an I18N-001 guard for this Command/Search surface.
- Bound the new guard in `docs/acceptance-cases/09_i18n.md` and `docs/dev-runbook.md`.

## Verification

Ran:

- `bash scripts/check-i18n-hardcoded-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `cargo test -p deve_web search_box -- --nocapture`
- `cargo fmt --check`
- `git diff --check`

Results: pass.

## Remaining Gaps

### G1. I18N Visible Text Batch 2

- Priority: P1.
- Plan basis: `11_i18n.md#i18n-facade-contract`.
- Scope: shell status, activity/sidebar controls, disconnect overlay, and remaining non-product visible labels.
- Constraint: keep product names, protocol constants, DOM markers and tests out of the cleanup target.

### G2. Release Dependency Audit Gate

- Priority: P2.
- Plan basis: `15_release.md`, `12_tech_release.md#REL-003`.
- Gap: `cargo audit` is listed in acceptance, but the current local environment has no `cargo-audit` binary.
- Required output: decide whether to add an explicit diagnostic guard, CI install step, or documented skip path.

### G3. Native Pre-Gate Freshness Report

- Priority: P2.
- Plan basis: `08_ui_design_02_desktop.md`, `08_ui_design_03_mobile.md`, `14_tech_stack.md#native-packaging-dependency-gate`.
- Gap: native packaging is correctly gate-closed, but a post-regression report should summarize Desktop/Mobile no-packaging shell, supervisor and recovery tests.
- Required output: report only unless tests expose drift.

## Decision

Proceed with G1 next. It is a Current MUST cleanup, small enough to avoid broad UI churn, and it directly improves the guard surface created in this batch.

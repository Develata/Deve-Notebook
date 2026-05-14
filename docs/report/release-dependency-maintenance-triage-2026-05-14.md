# Release Dependency Maintenance Triage - 2026-05-14

本报告记录 Mermaid moderate advisory 的维护处理。`docs/plan/` 仍是唯一权威；本文件只记录当前实现事实。

## Scope

- Plan basis: `15_release.md` release checklist and `14_tech_stack.md#graph-visualization`.
- Code scope: `apps/web/package.json`, `apps/web/package-lock.json`.
- Non-goal: open Graph renderer gate, change Mermaid runtime semantics, change release audit threshold, or edit `docs/plan/`.

## Finding

`npm audit --prefix apps/web --json` reported one moderate direct-dependency advisory group for Mermaid:

- `GHSA-6m6c-36f7-fhxh`: Mermaid Gantt infinite loop DoS.
- `GHSA-xcj9-5m2h-648r`: `classDefs` CSS injection.
- `GHSA-87f9-hvmw-gh4p`: configuration CSS injection.
- `GHSA-ghcm-xqfw-q4vr`: state diagram `classDef` HTML injection.

Affected range: `>=11.0.0-alpha.1 <=11.14.0`.

Current lock before this batch resolved `mermaid` to `11.13.0`; `npm view mermaid version` reported `11.15.0`, and audit marked a fix as available.

## Fixes

- Updated direct Web dependency range from `mermaid ^11.13.0` to `^11.15.0`.
- Updated lockfile to resolve `mermaid` to `11.15.0`.
- Kept the current Graph renderer gate closed; this is a dependency maintenance update for the existing Mermaid projection/widget surface, not a new graph-renderer capability.
- The new Mermaid parser dependency removes the previous `langium` / `vscode-languageserver*` transitive chain from the Web lockfile.

## Verification

Ran:

- `npm audit --prefix apps/web --audit-level=moderate`
- `npm --prefix apps/web run build`
- `scripts/smoke-web-release-build.sh`
- `bash scripts/check-release-audit-gate.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `cargo fmt --check`
- `git diff --check`

Results:

- npm audit: `found 0 vulnerabilities`.
- Web asset build: pass.
- Web release build: pass.
- Release audit gate: pass.
- Release baseline guard: pass.
- Plan coverage: `0` blocking violations, `17` existing soft size warnings.

## Residual Notes

- Local `check-release-audit-gate.sh` still skips `cargo audit` when `cargo-audit` is not installed. This is the existing diagnostic-mode behavior; CI required mode installs and runs it.
- This batch does not change `docs/plan/`; the next active item is still the error-code catalog drift review, which requires explicit permission before editing plan chapters.

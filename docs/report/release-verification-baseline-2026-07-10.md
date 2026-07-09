# First-tag Release Verification Snapshot - 2026-07-10

本报告记录首个公开 tag 前的一次本地 release evidence 刷新。它是时间戳快照，不替代
`docs/plan/18_release.md`、`docs/features/15_release.md`、
`docs/acceptance-cases/12_tech_release.md` 或 release workflow。

## Scope

- Branch: `main`
- Source base HEAD: `7bd177f733e8fed41c4b6f119f0eab71ab2b2262`
- GitHub Actions `Check` workflow run:
  <https://github.com/Develata/Deve-Notebook/actions/runs/29034859907>
- 本地 candidate patch 同步 release contracts、Tauri manifest versions 与
  metadata-less fail-closed test；本轮不创建 tag 或 release artifact。
- 截至本快照，没有本地/远端 Git tag，也没有 GitHub Release。

## Current Result

| Surface | Result | Evidence boundary |
|---|---|---|
| Branch check workflow | Passed for base HEAD | GitHub Actions `Check` workflow run completed with `success` at the source base HEAD above; candidate patch still requires final CI. |
| Deterministic release baseline | Passed | `scripts/check-release-baseline.sh` printed `release-baseline-check: ok`. |
| Plan reverse coverage | Passed | `scripts/plan-coverage.sh --check-reverse-coverage` reported zero blocking violations and `check-reverse-coverage: OK`. |
| Diff hygiene | Passed | `git diff --check` returned no diagnostics for the candidate patch. |
| Metadata-less repo test semantics | Contract tightened | The local missing-metadata database test now rejects `repo not found` as an acceptable corruption result; the targeted lib test passed. |
| Native manifest versions | Aligned | Desktop and Mobile Tauri manifests now match workspace version `0.1.0`. |
| Linux native Desktop first-tag route | Intentionally excluded | ADR 0006 Route 2 keeps GTK3/WebKitGTK 4.x Linux native artifacts outside the first-tag release set. |
| Docker/Web/CLI tag workflow | Not run | `.github/workflows/release.yml` requires a `v*` tag; this snapshot does not create one. |
| Windows/macOS/Android tag workflow | Not run | `.github/workflows/release-native.yml` requires a `v*` tag; this snapshot does not create one. |
| Published release artifacts | Not verified | No GitHub Release or tag-triggered GHCR/native artifact result exists yet. |

## Changelog Boundary

`CHANGELOG.md` currently contains a concrete `0.1.0` section and a later
`Unreleased` S3-compatible Remote Projection profile entry. Before creating the
first public tag, the selected tag version and release date must be explicit,
and every change that will ship in that tag must be moved into the matching
version section. This snapshot deliberately does not guess the final tag date
or silently rewrite version history.

## Evidence Not Claimed

This snapshot does not claim:

- successful execution of the tag-triggered `release.yml` or
  `release-native.yml` workflows;
- published or pull-tested GHCR images;
- signed native artifacts, store readiness, or physical-device readiness;
- Linux GTK3/WebKitGTK 4.x native package readiness;
- release artifact installation, upgrade, rollback, or uninstall evidence;
- stable data compatibility beyond the contracts already fixed by the first-tag
  format matrix and release baseline.

## Required Before Tag

1. Select the exact first public tag version and release date.
2. Reconcile `CHANGELOG.md` so all shipping changes belong to that version.
3. At the final tag candidate HEAD, re-run the current branch checks and the
   tag-ready dependency audit:
   `DEVE_RELEASE_TAG_READY_REQUIRED=1 scripts/check-release-audit-gate.sh`.
4. Create the tag only with explicit release authorization.
5. Verify both tag-triggered workflows, the GitHub Release, and published
   container/native artifact surfaces instead of treating branch CI as release proof.

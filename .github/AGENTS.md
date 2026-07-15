<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# .github

## Purpose

GitHub CI/CD configuration. The current first-tag baseline builds and smokes an
exact-HEAD candidate before any tag, seals it with receipts, then uses
`release.yml` only to promote those already verified bytes. Optional manual
target-host workflows may exist for diagnostics; nightly and speckit sync
workflows are not required repo metadata.

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `workflows/` | GitHub Actions workflow YAML files |

## Key Files (in workflows/)

| File | Description |
|------|-------------|
| `workflows/check.yml` | Branch push / PR check-only CI; no packaging, publishing, GHCR, or production actions |
| `workflows/release-candidate.yml` | Manual exact-HEAD quality, Docker, native, SBOM, attestation and candidate sealing workflow |
| `workflows/release.yml` | Sole tag-triggered promotion workflow; downloads an aggregate-bound sealed candidate and never rebuilds |
| `workflows/release-native.yml` | Reusable Windows/macOS/Android build and target-host smoke track; never publishes |
| `workflows/docker-smoke.yml` | Optional manual Docker release smoke on a GitHub-hosted Linux runner |
| `workflows/native-target-host.yml` | Optional manual Desktop/Mobile target-host diagnostics |
| `workflows/acceptance-aggregate.yml` | Manual exact-HEAD candidate/receipt verifier and sealed tag-ready bundle producer |

## For AI Agents

### Working In This Directory

- Workflow files use GitHub Actions YAML syntax.
- Keep workflows lean — the target environment is resource-constrained.
- Do not attach optional target-host workflows to tag releases. Direct tag
  triggering belongs only to `release.yml`; `release-native.yml` remains a
  build/smoke-only reusable workflow called by `release-candidate.yml`.
- Candidate and aggregate artifacts are immutable. Failed or partial runs must
  be replaced by a fresh workflow dispatch/run ID, never overwritten in place.
- Keep Docker smoke manual-only unless the release baseline explicitly promotes
  it into a required branch or tag gate.
- Do not recreate `nightly.yml` or `speckit-sync-check.yml` unless the plan
  explicitly reintroduces them as required release surfaces.

<!-- MANUAL: -->

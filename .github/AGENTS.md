<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# .github

## Purpose

GitHub CI/CD configuration. The current release baseline contains the
tag-driven release workflow only. Optional manual target-host workflows may
exist for deferred delivery evidence; nightly and speckit sync workflows are
not required repo metadata.

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `workflows/` | GitHub Actions workflow YAML files |

## Key Files (in workflows/)

| File | Description |
|------|-------------|
| `workflows/check.yml` | Branch push / PR check-only CI; no packaging, publishing, GHCR, or production actions |
| `workflows/release.yml` | Required release quality gates and Docker publishing |
| `workflows/docker-smoke.yml` | Optional manual Docker release smoke on a GitHub-hosted Linux runner |
| `workflows/native-target-host.yml` | Optional manual Desktop/Mobile target-host diagnostics |

## For AI Agents

### Working In This Directory

- Workflow files use GitHub Actions YAML syntax.
- Keep workflows lean — the target environment is resource-constrained.
- Do not attach optional target-host workflows to tag releases until the release
  plan explicitly promotes them into the required release baseline.
- Keep Docker smoke manual-only unless the release baseline explicitly promotes
  it into a required branch or tag gate.
- Do not recreate `nightly.yml` or `speckit-sync-check.yml` unless the plan
  explicitly reintroduces them as required release surfaces.

<!-- MANUAL: -->

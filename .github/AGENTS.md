<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# .github

## Purpose

GitHub CI/CD configuration. Contains workflow definitions for nightly builds, releases, and specification sync checks.

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `workflows/` | GitHub Actions workflow YAML files |

## Key Files (in workflows/)

| File | Description |
|------|-------------|
| `workflows/nightly.yml` | Nightly build and test workflow |
| `workflows/release.yml` | Release packaging and publishing |
| `workflows/speckit-sync-check.yml` | Specification document sync validation |

## For AI Agents

### Working In This Directory

- Workflow files use GitHub Actions YAML syntax.
- Keep workflows lean — the target environment is resource-constrained.

<!-- MANUAL: -->

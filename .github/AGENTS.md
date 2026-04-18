<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# .github

## Purpose

GitHub CI/CD configuration. The current baseline contains the tag-driven
release workflow only; nightly and speckit sync workflows are not required
repo metadata.

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `workflows/` | GitHub Actions workflow YAML files |

## Key Files (in workflows/)

| File | Description |
|------|-------------|
| `workflows/release.yml` | Release packaging and publishing |

## For AI Agents

### Working In This Directory

- Workflow files use GitHub Actions YAML syntax.
- Keep workflows lean — the target environment is resource-constrained.
- Do not recreate `nightly.yml` or `speckit-sync-check.yml` unless the plan
  explicitly reintroduces them as required release surfaces.

<!-- MANUAL: -->

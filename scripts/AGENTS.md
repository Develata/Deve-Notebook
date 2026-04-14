<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# scripts

## Purpose

Build and lint utility scripts for Deve-Notebook. Provides low-memory lint configurations for resource-constrained environments.

## Key Files

| File | Description |
|------|-------------|
| `lint-low-mem.cmd` | Windows CMD script — runs clippy with reduced memory |
| `lint-low-mem.ps1` | PowerShell script — runs clippy with reduced memory |
| `check-architecture-registry.sh` | Verifies operation registry, drift map, Lisp IDs, and graph spines stay aligned |

## For AI Agents

### Working In This Directory

- Scripts target Windows (CMD/PowerShell) since primary dev environment is WSL on Windows.
- Keep scripts minimal and focused on a single task.

<!-- MANUAL: -->

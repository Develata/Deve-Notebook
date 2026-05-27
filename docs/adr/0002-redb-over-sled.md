# 0002. redb over sled for embedded storage

- Status: Accepted
- Date: 2026-05-27

## Context

The ledger-first storage layer needs an embedded, transactional key-value store
on native targets to hold append-only ledger facts, snapshots, and side tables.
Candidates were sled (popular, but with a long-unstable on-disk format and
higher memory overhead) and redb (zero-copy reads, stable single-file format,
explicit ACID transactions).

## Decision

Use **redb** as the native embedded store. Zero-copy reads and a compact
single-file-per-repo-instance layout fit the `repo_name.redb` instance model and
the `low-spec` memory budget; explicit transactions match the controlled
ledger-append write path.

## Consequences

- Ledger tables, sequence contract, and side tables are defined against redb table schemas.
- redb table schema versioning is distinct from plan-chapter `Version` and protocol version.
- Commits the project to redb's transaction/table API rather than sled's tree API.

## References

- docs/plan/03_storage/authority.md (Storage Tables and Indexes)
- docs/plan/17_tech_stack.md (Storage)

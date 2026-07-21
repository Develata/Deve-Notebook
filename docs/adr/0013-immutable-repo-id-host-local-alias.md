# 0013. Immutable RepoId and host-local repo alias

- Status: Accepted
- Date: 2026-07-18

> Decision item 7's unpublished F4/v4 target is superseded by ADR 0014 F4/v5.
> RepoId and host-local alias decisions remain accepted.

## Context

The previous repository contract treated the current repo name as a
ledger-derived binding and also derived the physical workspace directory from
that mutable value. In practice the implementation stored a mutable
`RepoInfo.name`, moved the workspace during rename, and coupled a human-facing
label to watcher shutdown, locator repair, Remote Import lifecycle and
multi-peer semantics.

For a Markdown notebook, cross-host correctness depends on the immutable repo
identity and the complete Markdown/Ledger facts. A user's preferred label for
the same repo is local human-interface state. Synchronizing it adds conflict,
migration and recovery paths without improving document fidelity.

No formal version has been released, so the unpublished `RepoNameBinding`,
`repo_name_hint`, workspace-move rename and related wire shapes can be removed
without compatibility adapters.

## Decision

1. `RepoId` is the immutable cross-host logical identity. Peers exchange no repo
   alias and still verify genesis/Ledger identity and authenticated source;
   UUID collision resistance is not authorization.
2. The mutable display name becomes a host-local
   `HostRepoAliasBinding { repo_id, alias, alias_revision }`, exclusively owned
   by `host_repo_alias_runtime`. Missing alias displays the full RepoId.
3. Alias changes never append Ledger facts, alter sync/Remote Import/provider
   state, stop a watcher, move a workspace or create Projection Fault evidence.
4. Projection Locator owns an immutable `workspace_segment`. Local creation may
   preserve a readable `<safe_initial_alias>--<repo_id>` segment; a first local
   binding without alias uses `<repo_id>`. Later alias changes do not rewrite it.
5. Alias JSON uses deterministic version 1 with only
   `format/version/aliases[{repo_id,alias}]`. Import is dry-run by default.
   Unknown local RepoId, invalid alias, duplicate RepoId and per-entry admission
   failures are warnings and are skipped with a complete final summary. All
   accepted entries commit atomically; store-wide failure is a global error.
6. Durable create/remove lifecycle adopts a host-owned job runtime and
   prepare/short-authority-cut/settle phases. Transport cancellation cannot
   cancel a received job. A committed cut produces an immutable committed-cut
   plan; settlement then produces a distinct immutable publication carrying the
   final mount/cleanup outcome.
7. The still-unpublished F4/v3 epoch is replaced by F4/v4 lockstep. Repo Control
   becomes one nested typed family; old name selector/Create/Rename/Remove wire
   variants are deleted without adapters.

## Consequences

- Multi-peer data processing and local human interaction remain maximally
  independent without weakening RepoId admission, Ledger authority or Markdown
  fidelity.
- Duplicate local aliases are allowed; ambiguous alias selectors fail closed.
- Users who want the same naming scheme on several hosts explicitly export and
  import the small JSON mapping. No peer silently imposes its labels.
- Physical workspace paths remain stable while users freely rename aliases.
- The current Redb/Ledger payload, sync facts and Projection file format do not
  need a new cross-host compatibility layer.
- The repo lifecycle implementation must delete handler-owned durable futures,
  long Catalog/Repo critical sections and post-commit ordinary errors before W7
  can be signed complete.

## References

- docs/plan/01_terminology.md
- docs/plan/03_storage/index.md
- docs/plan/03_storage/projection.md
- docs/plan/04_repository.md
- docs/plan/07_network.md
- docs/plan/14_commands.md
- docs/report/repo-lifecycle-w7-architecture-stop-2026-07-18.md

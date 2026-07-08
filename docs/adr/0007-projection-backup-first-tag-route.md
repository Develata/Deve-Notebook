# 0007. Projection Backup first-tag route

- Status: Accepted
- Date: 2026-07-08

## Context

The previous Backup plan treated WebDAV/S3 as an authority-grade ledger backup
surface with separate encrypted artifacts, manifests, restore-candidate
admission, and ledger import/merge paths. That route duplicates durable history
that NoteGit/ngit + Git remote already owns, and it turns a simple Markdown
transport feature into a separate disaster-recovery subsystem.

The accepted product intent is narrower and cleaner:

- Backup means transferring Markdown Projection Workspace files to/from
  WebDAV/S3/S3-compatible storage.
- Restoring from remote writes only Projection Workspace files.
- The restored file changes then surface as External Changes.
- Ledger writes occur only after user confirmation through the existing External
  Changes / Source Control authority path.
- Ledger history durability belongs to NoteGit/ngit + Git remote, not to
  WebDAV/S3 Backup.

## Decision

Adopt **Projection Backup** for the first formal tag.

Projection Backup is the backup-oriented product name for Remote Projection
Transport. It reuses the WebDAV/S3 provider adapters, locator admission,
Projection Workspace gates, provider metadata diagnostic-only contract, and S3
custom endpoint credential-profile fail-closed posture.

The following authority-grade ledger backup capabilities are removed from the
first-tag product contract: independent ledger backup artifacts/manifests,
restore-candidate admission, ledger import/merge runtime, and WebDAV/S3
ledger-history disaster recovery.

Any future ledger backup feature must be reintroduced by a new ADR and an
independent runtime proposal. It must not be smuggled back through Projection
Backup or Remote Projection Transport.

## Rationale

Projection Backup matches the actual user value: move Markdown files between a
local Projection Workspace and object/file storage. It preserves the simple
truth table:

| Surface | Authority |
|---|---|
| Markdown file transport | Projection Backup / Remote Projection Transport |
| File-to-ledger admission | External Changes user confirmation |
| Durable history | NoteGit/ngit + Git remote |
| Provider metadata | diagnostics only |

This is more pure and better optimized than maintaining a parallel encrypted
ledger pack format. It avoids duplicate history systems, keeps provider I/O out
of authority, and uses already-existing External Changes semantics for user
review.

## Consequences

- `docs/plan/06_backup.md` is rewritten around Projection Backup.
- Feature docs, acceptance cases, registries, and baseline specs must stop
  presenting ledger pack restore as first-tag scope.
- Legacy `backup` CLI/code may either be removed or converted to fail-closed
  guidance; it must not remain a first-tag release gate.
- Remote Projection WebDAV/S3 push/pull tests become the relevant automated
  evidence for Projection Backup.
- S3-compatible custom endpoints remain governed by ADR 0008 and must be bound
  to explicit Remote Projection profile semantics before provider I/O.

## References

- docs/plan/06_backup.md
- docs/plan/05_diff_logic.md
- docs/features/06_repository.md
- docs/acceptance-cases/07_storage_repo.md
- docs/registry/runtime-skeleton-registry.md
- docs/adr/0008-s3-compatible-remote-projection-credential-binding.md

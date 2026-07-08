# 0008. S3-compatible Remote Projection credential binding

- Status: Proposed
- Date: 2026-07-07

## Context

Remote Projection transports Markdown Projection Workspace files through
WebDAV or S3-compatible object storage. It is the transport runtime used by
Projection Backup, but it is not ledger backup, sync authority, Source Control
authority, ledger authority, or Git mirror authority. `push`
uploads only projection Markdown files. `pull` writes only Projection
Workspace files and then relies on watcher/scan External Changes admission.

The current S3 provider supports AWS `s3://bucket/prefix` with explicit runtime
environment credentials and SigV4 object operations. It intentionally rejects
`s3+https://` custom endpoints before provider I/O and before resolving
default `AWS_*` credentials. The Web Command Palette sends only provider and
direction typed intents; backend resolves the locator from the current local
repo `repo_url`.

Allowing custom endpoints without a credential binding/profile contract would
let any repo-local locator cause the runtime to sign arbitrary hosts with
default AWS environment keys. That is a security and authority boundary change,
not a provider URL parsing patch.

The current safe direction is to keep custom endpoint I/O fail-closed until a
dedicated Remote Projection profile contract is accepted and implemented. The
remaining decision questions are:

- Where the custom endpoint allowlist and credential reference live.
- How to prove that a locator origin, bucket, and root prefix match a binding.
- Whether CLI can use explicit profile names before the Web path is enabled.
- How to ensure default AWS environment credentials are never reused for an
  arbitrary custom endpoint.
- How to keep Remote Projection binding separate from any future ledger backup
  binding and from repo authority data.

## Decision

No implementation route is accepted yet. Before `s3+https://` provider I/O can
be enabled, the project must choose one of these routes:

1. **Dedicated Remote Projection provider profile runtime.**
   Add a host-local, secret-free Remote Projection profile store that is
   separate from any future ledger backup binding and separate from ledger authority. A profile
   binds provider `s3-compatible`, endpoint origin, bucket, allowed root
   prefix, region/signing settings, and a credential reference. The credential
   reference points to a runtime secret source, but raw access keys, secret
   keys, and session tokens are never persisted in repo metadata, Projection
   Workspace, Backup metadata, or Web client state. `s3+https://` provider I/O
   is allowed only when the current locator matches an active profile. Web
   continues to submit typed provider/direction intents; backend re-resolves
   repo scope, locator, and profile match before every operation.

2. **CLI-only explicit profile route first.**
   Keep Web `s3+https://` fail-closed, but allow CLI `projection-remote s3`
   operations to pass an explicit Remote Projection profile name. The profile
   still uses a dedicated host-local binding store and does not accept raw
   secrets or default AWS environment keys for custom endpoints. This proves
   provider I/O behavior before a browser-visible profile management flow
   exists.

3. **Defer S3-compatible endpoints from the first formal tag.**
   Keep `s3+https://` fail-closed and ship only WebDAV plus AWS `s3://` S3
   Remote Projection until a complete binding/profile UX is accepted.

The recommended route is **Route 1**, with Route 2 acceptable as a smaller
implementation slice only if it writes the same profile contract that Route 1
will later use.

## Rationale

Route 1 is the cleanest long-term boundary. It keeps endpoint admission,
credential selection, and provider I/O in backend/runtime infrastructure while
preserving the frontend thin-shell rule. It also gives Web, CLI, and future
mobile shells the same security model: user intent selects provider/direction,
runtime authority resolves locator/profile and performs transport.

Route 2 lowers implementation risk by avoiding immediate browser-visible
profile management, but it must not create a CLI-only credential model that the
Web path later has to bypass or replace. It is useful only as a staged slice of
Route 1.

Route 3 is operationally safe, but it leaves R2, MinIO, and other
S3-compatible endpoints unavailable for the first formal release. It is
acceptable only if the first tag intentionally narrows Remote Projection to
WebDAV and AWS S3.

## User Impact

With Route 1, users can configure a custom endpoint once, then use the same
Remote Projection actions from Web or CLI without exposing secrets to the
frontend. If a repo points at an unbound endpoint, the operation remains
fail-closed with a clear `provider_io_ready=false` diagnostic.

With Route 2, users can validate R2/MinIO-style provider I/O from CLI, but Web
Command Palette actions for custom endpoints still fail closed until profile
management is available.

With Route 3, users can use WebDAV or AWS S3 only. Custom endpoint users must
wait, but the first release avoids locking in an unsafe credential path.

## Consequences

- Current behavior does not change. `s3+https://` remains fail-closed until an
  implementation route is accepted and implemented.
- Default `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`,
  `AWS_REGION`, and `AWS_DEFAULT_REGION` must not be used for custom endpoint
  provider I/O unless an accepted profile contract explicitly binds that secret
  source to the endpoint.
- Remote Projection profiles must not reuse future ledger-backup binding stores,
  artifact metadata, or provider adapters as sync/source-control authority.
- Profile data must be host-local and secret-free. It may identify a credential
  reference, endpoint origin, bucket, prefix, region, and signing options, but
  not raw secret values.
- `push` and `pull` authority rules stay unchanged: only Markdown projection
  files are transported; `pull` writes Projection Workspace files and enters
  External Changes; no ledger, staging, commit anchor, Git mirror queue, backup
  state, or Source Control authority state is written by provider metadata.
- Frontend and Command Palette surfaces must not accept locator strings,
  endpoint URLs, access keys, secret keys, session tokens, ETags, or provider
  metadata as operation authority. They may only collect typed user intent or,
  after an accepted profile UX exists, select backend-defined profile handles.

## References

- docs/plan/05_diff_logic.md
- docs/plan/14_commands.md
- docs/features/07_diff_logic.md
- docs/features/12_commands.md
- docs/acceptance-cases/04_diff.md
- docs/registry/runtime-skeleton-registry.md

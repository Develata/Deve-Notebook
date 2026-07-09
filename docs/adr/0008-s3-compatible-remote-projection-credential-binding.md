# 0008. S3-compatible Remote Projection credential binding

- Status: Accepted
- Date: 2026-07-09

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

Accept the **Dedicated Remote Projection provider profile runtime** as the
long-term route. First-tag pressure must not introduce a shortcut that signs an
arbitrary S3-compatible endpoint with default AWS environment credentials.

The accepted model is:

1. **Host-local, secret-free profile store.** A Remote Projection profile is
   local to the current host and separate from ledger authority, Projection
   Locator, Source Control, Git mirror, sync, and any future ledger-backup
   binding. It stores only binding metadata: stable profile handle, provider
   kind `s3-compatible`, HTTPS endpoint origin, bucket, allowed root prefix,
   region/signing scope, addressing-style/capability flags, allowed directions,
   and a credential reference.
2. **Runtime credential resolver.** The credential reference points to a runtime
   secret source such as an explicitly named environment-variable set, OS/keyring
   secret, or future host secret adapter. Raw access keys, secret keys, and
   session tokens are never persisted in repo metadata, Projection Workspace,
   locator strings, Backup metadata, Web client state, normal logs, or crash
   reports.
3. **Exact binding admission.** `s3+https://` provider I/O is allowed only when
   the operation locator matches an active profile by provider kind, normalized
   endpoint origin, bucket, and prefix containment. Region/signing settings must
   be explicit in the profile; provider-specific values such as Cloudflare R2
   `auto` are allowed only when pinned by the profile.
4. **No ambient AWS fallback for custom endpoints.** Process-wide
   `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`,
   `AWS_REGION`, and `AWS_DEFAULT_REGION` are not used for `s3+https://` unless
   an explicit profile credential reference intentionally points at that secret
   source and the endpoint/bucket/prefix match that same profile.
5. **Thin Web surface.** Web / Command Palette continues to submit typed user
   intent only. It may select a backend-defined profile handle after a profile UX
   exists, but it never accepts endpoint URLs, locator strings, access keys,
   secret keys, session tokens, ETags, or provider metadata as authority.

A **CLI-only explicit profile slice** is acceptable as an implementation stage
only if it writes and validates this same profile contract. It must not create a
parallel CLI-only credential model. A release-driven route that merely defers
S3-compatible endpoints remains safe as current behavior, but it is no longer
the architectural target.

Until the accepted profile runtime is implemented and verified, `s3+https://`
custom endpoint I/O remains fail-closed before provider I/O and before default
AWS credential resolution.

## Rationale

The accepted route is the cleanest long-term boundary. It keeps endpoint admission,
credential selection, and provider I/O in backend/runtime infrastructure while
preserving the frontend thin-shell rule. It also gives Web, CLI, and future
mobile shells the same security model: user intent selects provider/direction,
runtime authority resolves locator/profile and performs transport.

The CLI-only slice lowers implementation risk by avoiding immediate browser-visible
profile management, but it must not create a CLI-only credential model that the
Web path later has to bypass or replace. It is useful only as a staged slice of
the accepted route.

Deferring provider I/O is operationally safe while the profile runtime is still
missing, but it should not erase the long-term S3-compatible design merely to
accelerate the first formal tag.

## User Impact

With the accepted profile route, users can configure a custom endpoint once, then use the same
Remote Projection actions from Web or CLI without exposing secrets to the
frontend. If a repo points at an unbound endpoint, the operation remains
fail-closed with a clear `provider_io_ready=false` diagnostic.

With the CLI-only slice, users can validate R2/MinIO-style provider I/O from CLI, but Web
Command Palette actions for custom endpoints still fail closed until profile
management is available.

While the profile runtime is deferred, users can use WebDAV or AWS S3 only.
Custom endpoint users must wait, but the first release avoids locking in an
unsafe credential path.

## Consequences

- Current behavior does not change. `s3+https://` remains fail-closed until the
  accepted profile runtime is implemented and verified.
- Default `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`,
  `AWS_REGION`, and `AWS_DEFAULT_REGION` must not be used for custom endpoint
  provider I/O unless an accepted profile contract explicitly binds that secret
  source to the endpoint.
- Remote Projection profiles must not reuse future ledger-backup binding stores,
  artifact metadata, or provider adapters as sync/source-control authority.
- Profile data must be host-local and secret-free. It may identify a credential
  reference, endpoint origin, bucket, prefix, region, signing options,
  addressing style, provider capability flags, and allowed directions, but not
  raw secret values.
- `push` and `pull` authority rules stay unchanged: only Markdown projection
  files are transported; `pull` writes Projection Workspace files and enters
  External Changes; no ledger, staging, commit anchor, Git mirror queue, backup
  state, or Source Control authority state is written by provider metadata.
- Frontend and Command Palette surfaces must not accept locator strings,
  endpoint URLs, access keys, secret keys, session tokens, ETags, or provider
  metadata as operation authority. They may only collect typed user intent or,
  after a profile UX exists, select backend-defined profile handles.

## References

- docs/plan/05_diff_logic.md
- docs/plan/14_commands.md
- docs/features/07_diff_logic.md
- docs/features/12_commands.md
- docs/acceptance-cases/04_diff.md
- docs/registry/runtime-skeleton-registry.md

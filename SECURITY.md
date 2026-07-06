# Security Policy

## Supported Versions

Deve-Notebook is still pre-release. Until the first public stable release, the
supported security target is the current `main` branch plus any release
candidate branch explicitly named in release notes.

After the first stable release, this file must be updated with the supported
version window before publishing artifacts.

## Reporting a Vulnerability

Report suspected vulnerabilities privately through GitHub Private Vulnerability
Reporting or a draft GitHub Security Advisory for this repository:

https://github.com/Develata/Deve-Notebook/security/advisories/new

Do not open a public issue or discussion for an undisclosed vulnerability. If the
private GitHub advisory channel is unavailable, the release is blocked until the
maintainers publish an equivalent private contact channel.

Useful reports include:

- affected version, branch, or commit;
- affected deployment mode, such as CLI server, Web client, native shell,
  WebDAV/S3 projection transport, backup runtime, or plugin/trusted-agent path;
- reproduction steps and expected impact;
- whether secrets, ledger authority, backup artifacts, or source-control state
  may have been exposed or modified.

## Response Targets

These are target service levels, not legal guarantees.

| Step | Target |
|---|---|
| Acknowledge receipt | 3 business days |
| Initial severity assessment | 7 business days |
| Critical or high-impact mitigation plan | 14 calendar days |
| Medium or low-impact fix plan | next planned maintenance window |

Issues that can compromise ledger authority, authentication/session integrity,
backup confidentiality, provider credentials, plugin/native execution policy, or
release artifact integrity are treated as T1 until triage proves otherwise.

## Coordinated Disclosure

Reports remain private until a fix, mitigation, or documented non-issue decision
is ready. Public disclosure should happen through a release note, GitHub
Security Advisory, CVE entry when applicable, and optional reporter credit.

If exploitation is active or likely, maintainers may publish a limited advisory
before a full fix to reduce user harm.

## Key Lifecycle Policy

Secret material must not be committed, logged, embedded in URLs, stored in
browser storage, or written into projection workspaces.

Key custody follows the owning runtime contracts:

- auth JWT secret and password hash policy: `docs/plan/08_auth.md`;
- P2P admission token material: `docs/plan/07_network.md`;
- peer identity keys: `crates/core/src/security/`;
- backup encryption key references: `docs/plan/06_backup.md`;
- native shell and local backend bootstrap: `docs/plan/11_ui_design/`.

Rotation is required after suspected exposure, maintainer or deployment trust
changes, provider compromise, or algorithm deprecation. If a rotation or revoke
protocol is not yet defined by the owning contract, the project must fail closed
for that release path instead of inventing an ad hoc migration.

## Algorithm Deprecation Policy

Cryptographic algorithm retirement follows `docs/plan/23_threat_model.md`.
Removing an auth token, password hash, peer signature, backup artifact, or
transport protection algorithm requires an owner contract update, a migration
window where old and new material can be handled safely, and a release gate that
proves the old algorithm is no longer required.

If the owning contract has not defined a migration or verification path, the
release path must fail closed instead of silently dropping compatibility or
accepting unverifiable material.

## Supply Chain Policy

Release artifacts must be built from the checked-in lockfile and include or link
to a dependency inventory derived from `Cargo.lock` and `cargo metadata` or
`cargo tree`.

Before adding new dependencies, review license, maintenance status, transitive
dependency weight, native build requirements, and low-memory deployment impact.
Native packaging and platform dependencies remain gated by
`docs/plan/17_tech_stack.md`.

Security-relevant release gates include formatting, linting, targeted tests,
plan/acceptance baseline checks, dependency/native gate checks, and artifact
signing rules owned by `docs/plan/18_release.md`.

# 0010. Sealed pre-tag release candidate promotion

- Status: Accepted
- Date: 2026-07-15

## Context

The former first-tag workflow built Docker and native artifacts only after a tag
was pushed. Target-host receipts could therefore prove the source revision while
the bytes eventually published by the tag workflow were newly rebuilt. A file
name inventory also could not prove artifact identity, SBOM subjects, checksums,
or provenance.

## Decision

Build, smoke, hash, attest, and seal the complete first-tag candidate before the
tag exists. The candidate workflow binds every artifact and receipt to one HEAD,
version, workflow run, Docker image ID, and Android signer. The aggregation
workflow verifies the candidate and platform receipts and emits one sealed
bundle. The tag-triggered release workflow may only load and promote that bundle;
it may not rebuild, repackage, rename, or silently select a different run.
Candidate and aggregate runs are single-attempt: a failure requires a fresh
dispatch/run ID because overwriting an artifact behind a tag-bound run ID would
destroy immutability.

The source/workspace SBOM and Docker image SBOM remain distinct subjects. A
source SBOM is never represented as a byte-level SBOM for MSI, DMG, or APK.
Public checksums use the actual flattened Release asset basenames. Provenance and
Docker SPDX bundles are distinct typed inputs and are reverified against the
signer workflow and source HEAD; the sealed APK signer is independently
re-extracted. Host-only macOS packages retain their real x64/arm64 identity.

The Git tag and manifest preserve full SemVer. Because Docker tags reject `+`,
build metadata is injectively encoded as `_build_`; prereleases never mutate
`latest`, and stable `latest` advances only with both SemVer precedence and Git
ancestry.

Remote existence checks are three-state. Only an explicit HTTP 404 means
absent; transport, authentication, rate-limit, and server errors stop promotion.
If publication committed but the runner later lost the outcome, a rerun may
accept an already-public Release only after the tag, complete asset set and
digests, release classification, and registry identity all match the same
sealed candidate. Promotion then idempotently reapplies the stable/latest or
prerelease classification so GitHub Latest cannot diverge from GHCR `latest`.

## Consequences

- Candidate artifacts consume more CI storage and must be regenerated after an
  artifact expires or the HEAD/version changes.
- Android signing secrets are required before candidate sealing rather than
  being an optional post-tag improvement.
- Tag publication is faster and proves byte identity, but registry and GitHub
  Release publication are still not a cross-service atomic transaction.
- Exact-candidate idempotency repairs committed-unknown outcomes without
  treating remote probe failures as absence or allowing different bytes.
- No product API, ledger, WS, PeerFactSeq, or Source Control authority changes.

## References

- docs/plan/18_release.md
- docs/plan/23_threat_model.md
- docs/registry/acceptance-matrix.tsv

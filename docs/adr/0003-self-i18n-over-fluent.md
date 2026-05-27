# 0003. Self-built i18n facade over Fluent

- Status: Accepted
- Date: 2026-05-27

## Context

User-visible text must be localizable, and backend failures must map to stable,
structured error codes (not natural-language strings). Mozilla Fluent is a full
ICU-style localization system but adds runtime weight and a bundle/resolver
model heavier than this product needs on `low-spec`.

## Decision

Use a **self-built i18n facade** (`crate::i18n::t::...`) plus a single
authoritative error-code catalog. UI text is fetched only through the facade;
backends return structured error codes that the frontend maps to localized
strings. Branch/decision logic depends on `code`, never on natural-language
`detail`.

## Consequences

- The error-code catalog (`13_i18n#i18n-error-code-catalog`) is the single source of truth for codes.
- No Fluent bundle/resolver dependency; locale assets stay light.
- Adding a locale or message is a facade/catalog change, not a Fluent resource migration.

## References

- docs/plan/13_i18n.md (i18n facade, Error Code Catalog)

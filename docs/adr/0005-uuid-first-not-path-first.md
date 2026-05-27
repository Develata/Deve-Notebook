# 0005. UUID-first identity, not path-first

- Status: Accepted
- Date: 2026-05-27

## Context

A notebook with rename/move and multi-peer sync must decide what identifies a
node and a document. A path-first model (file path is the identity) makes
rename/move destructive to identity and fragile under concurrent sync. The
alternative is stable opaque identifiers with path as a derived projection.

## Decision

Adopt **UUID-first identity**: `NodeId` (structure layer) and `DocId` (content
layer) are 128-bit UUID v4 authority keys; for a `File` node `doc_id == node_id`.
`Path` / `path_cache` / `TreeDelta` / `NodeMeta` are projections, never the
authoritative source. Rename/move changes only the path projection, not identity.

## Consequences

- Structure Facts key on `NodeId`; Content Facts key on `DocId`; the two are not interchangeable.
- Rename/Move/Create/Delete and source-control rename share one structure-fact write path.
- Path mapping `M: DocId ↔ FilePath` is fold-derived; metadata tables must not directly drive identity.

## References

- docs/plan/01_terminology.md (NodeId, DocId, Path Mapping)
- docs/plan/04_repository.md (node-first model)

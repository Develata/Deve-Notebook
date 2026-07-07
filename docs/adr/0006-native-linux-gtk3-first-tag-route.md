# 0006. Native Linux GTK3 first-tag route

- Status: Proposed
- Date: 2026-07-07

## Context

The first formal public tag is expected to include Desktop and Mobile, while
the current gated Tauri native-packaging stack still resolves the Linux GTK3
family and `glib` warning set:

- `atk`, `atk-sys`, `gdk`, `gdk-sys`, `gdkwayland-sys`, `gtk`, `gtk-sys`,
  `gtk3-macros`: unmaintained GTK3 stack.
- `glib`: unsound warning on the resolved 0.18 line.

These warnings are behind the optional `native-packaging` scope and do not give
the shell direct ledger, Projection Workspace, Source Control, search, `.git`,
or `.notegit` authority. They are still first-tag blockers because a formal
Desktop/Mobile release would make the native shell a shipped artifact, not a
developer-only spike.

## Decision

No route is accepted yet. Before the first formal public tag, the project must
choose exactly one of these routes:

1. **Upgrade or replace the native shell dependency stack.**
   Move the Linux native shell route to a maintained GTK/WebView dependency
   line, or another maintained shell implementation that still satisfies the
   existing native adapter contracts.

2. **Exclude Linux GTK3 native artifacts from the first formal tag.**
   Ship Windows/macOS native desktop evidence only where the target-host
   package, installer, signing and startup gates are independently proven, and
   keep Linux GTK3/glib native artifacts out of the formal first-tag release
   set.

3. **Explicit USER risk acceptance.**
   Retain the gated GTK3 Linux shell route for the first formal tag only after
   an explicit USER decision records the risk, scope, replacement follow-up and
   evidence boundary.

The recommended route is **Route 2 unless a maintained native stack upgrade is
small and already proven on the target hosts**.

## Rationale

Route 1 is the cleanest long-term route, but it may turn into a broad native
shell migration. It should not be started as a small cleanup item because it
can affect package build scripts, WebView behavior, target-host dependencies,
installer evidence and smoke tests.

Route 2 preserves first-tag quality without accepting stale Linux GTK3 risk.
It also keeps the core product boundaries intact: Web/Server/Docker and
non-Linux native evidence can move forward while Linux native packaging remains
blocked until a maintained stack is available.

Route 3 should be the last resort. It creates release debt immediately: future
security fixes, Linux installer support and support requests would all need to
carry an explicit compatibility and advisory story from the first tag onward.

## User Impact

- Route 1 gives users a complete Linux native story if the migration is proven,
  but it may delay the first tag and can introduce new native-shell regressions.
- Route 2 means Linux users use Web/Server/Docker for the first formal tag
  rather than a native GTK3 package. This is honest and avoids promising a
  risky artifact.
- Route 3 gives the broadest artifact list immediately, but users receive a
  native Linux artifact with known advisory debt. This increases support and
  compatibility burden after the first tag.

## Consequences

- `release-audit-gate tag-ready` must continue to fail while the GTK3/glib
  rows in `docs/registry/release-audit-warning-registry.md` have
  `tag_blocker=yes`.
- This ADR does not open native authority writes, process runtime outside the
  existing gates, Linux package publication, store readiness, signing readiness
  or physical-device readiness.
- Any implementation of Route 1 or Route 2 must update the release plan,
  feature docs, acceptance cases, warning registry and target-host evidence
  before removing the tag blocker.

## References

- docs/plan/11_ui_design/02_desktop.md
- docs/plan/11_ui_design/03_mobile.md
- docs/plan/17_tech_stack.md
- docs/plan/18_release.md
- docs/features/15_release.md
- docs/registry/release-audit-warning-registry.md

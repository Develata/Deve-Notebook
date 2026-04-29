# Native Packaging Dependency Gate - 2026-04-29

## Result

P3-10 native packaging dependency gate has landed.

Implemented scope:

- `apps/desktop` declares a no-op `native-packaging` feature.
- `apps/mobile` declares a no-op `native-packaging` feature.
- Default desktop/mobile builds remain no-packaging skeletons.
- `scripts/check-native-track-boundary.sh` now fails if a packaging runtime
  dependency appears in any `Cargo.toml` before this gate is explicitly opened.
- The same script fails if app/core source code imports a packaging runtime
  before the dependency gate changes.
- Desktop, mobile, and tech-stack plans now point to
  `14_tech_stack.md#native-packaging-dependency-gate`.

Boundary:

- No packaging runtime dependency was added in this batch.
- `native-packaging` is only a named future gate. It does not change runtime
  behavior and does not grant native shell authority over ledger, vault,
  source-control, search, `.git`, or `.notegit`.
- Future packaging work must keep dependencies isolated to `apps/desktop` or
  `apps/mobile`; workspace root, `deve_core`, `deve_cli`, and `deve_web` remain
  dependency-free with respect to native packaging.

## Verification

Commands run:

```bash
scripts/check-native-track-boundary.sh
cargo test -p deve_desktop --all-features
cargo test -p deve_mobile --all-features
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo fmt --all --check
scripts/plan-coverage.sh
git diff --check
```

Observed result:

- Native boundary script passed.
- Desktop all-features tests passed.
- Mobile all-features tests passed.
- Workspace all-targets all-features check passed.
- Workspace all-targets all-features tests passed.
- Plan coverage passed with zero blocking violations.

## Next Work

The desktop packaging scaffold split was completed in
`desktop-packaging-scaffold-status-2026-04-29.md`.

The mobile packaging scaffold split was completed in
`mobile-packaging-scaffold-status-2026-04-29.md`.

Do not open the real Tauri dependency gate by default. The next native-track
step should be selected explicitly from packaging runtime adoption, embedded
service supervision, or platform bridge work after a focused review.

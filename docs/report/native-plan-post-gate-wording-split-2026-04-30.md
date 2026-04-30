# Native Plan Post-Gate Wording Split - 2026-04-30

## Scope

Closed the P0 docs drift where desktop/mobile plan chapters mixed current no-Tauri skeleton status with future Tauri/embedded-service/offline-first MUST language.

## Change

- Desktop and Mobile plan headers now state the current native boundary first: no-Tauri skeleton, no real child-process runtime, contract-only adapter/session/readiness/recovery validation.
- Tauri/Tauri Mobile and full offline-first language is now labeled as a post-gate target.
- Implementation Strategy sections now explicitly say their Tauri/embedded-service rules are post-gate normative targets, not current release acceptance.
- `scripts/check-native-track-boundary.sh` now guards the split wording so future edits do not collapse current/future native status again.

## Verification

- `scripts/check-native-track-boundary.sh`

## Remaining Work

- Graph blocked/degraded acceptance polish remains the next active queue item.

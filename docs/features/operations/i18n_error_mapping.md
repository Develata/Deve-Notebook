# i18n_error_mapping.md - I18n 错误码映射链

## Metadata

- `Flow ID`: `flow.i18n.error-mapping`
- `Domain`: `i18n`
- `Related Feature Chapters`: `docs/features/11_i18n.md`, `docs/features/09_auth.md`
- `Related Acceptance Cases`: `I18N-004`, `I18N-006`, `AUTH-002`

## Operations

### `op.i18n.error.receive-auth-code`

- `Name`: `Receive Auth Error Code`
- `Surface`: `api-response`
- `Trigger`: auth API rejects a request with a structured code
- `Preconditions`: server response includes a code field
- `Immediate Result`: frontend receives protocol-stable error identity
- `Application Entry`: `crates/core/src/protocol/auth.rs`, `apps/web/src/i18n/server_error.rs`

### `op.i18n.error.receive-sc-code`

- `Name`: `Receive Source Control Error Code`
- `Surface`: `api-or-websocket`
- `Trigger`: source-control or sync operation fails
- `Preconditions`: backend emits `ServerErrorCode`
- `Immediate Result`: frontend routes by code, not natural-language detail
- `Application Entry`: `crates/core/src/protocol/error.rs`, `apps/web/src/hooks/use_core/effects/message_protocol.rs`

### `op.i18n.error.render-message`

- `Name`: `Render Localized Error Message`
- `Surface`: `workspace-shell`
- `Trigger`: frontend maps `ServerErrorCode` through i18n
- `Preconditions`: locale is selected
- `Immediate Result`: user sees localized text while protocol remains code-based
- `Application Entry`: `apps/web/src/i18n/server_error.rs`

## Response Flow

1. User triggers an auth, source-control, or sync error.
2. Instruction interface receives a structured protocol error code.
3. Flow coordination maps code through the locale catalog.
4. Execution domains are protocol and i18n.

## Notes

- Backend detail strings may exist for debugging, but UI semantics must use codes.
- Main objects: `protocol::error-code`, `i18n::catalog`, `locale::selection`.

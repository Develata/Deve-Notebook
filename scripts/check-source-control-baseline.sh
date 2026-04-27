#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "source-control-baseline-check: $*" >&2
  exit 1
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file"; then
    fail "unexpected '$pattern' in $file"
  fi
}

# DIFF-009: Command Palette Git actions remain Planned / Optional. Source Control panel
# owns the current stage/commit/publish entry points.
check_contains docs/plan/12_commands.md "Git Sync / Commit / Push 与 AI Retry / Backend / PLAN / BUILD 面板命令仍属于"
check_contains apps/web/src/components/command_palette/registry.rs "merge_peer_command"
check_absent apps/web/src/components/command_palette/registry.rs "Git: Sync"
check_absent apps/web/src/components/command_palette/registry.rs "Git: Commit"
check_absent apps/web/src/components/command_palette/registry.rs "Git: Push"
check_absent apps/web/src/i18n/command_palette.rs "Git: Sync"
check_absent apps/web/src/i18n/command_palette.rs "Git: Commit"
check_absent apps/web/src/i18n/command_palette.rs "Git: Push"

# Current Source Control panel operations are explicit and repo-scoped.
check_contains docs/plan/07_diff_logic.md "CommitStaged"
check_contains docs/features/operations/sc_commit.md 'ClientMessage::Commit { scope_nonce }'
check_contains apps/web/src/hooks/use_core/callbacks_sc_write_commit.rs "ClientMessage::Commit"
check_contains apps/web/src/hooks/use_core/callbacks_sc_write_commit.rs "ClientMessage::CommitAndPush"
check_contains apps/web/src/hooks/use_core/callbacks_sc_write_commit.rs "source_control_scope_nonce(scope)"
check_contains apps/cli/src/server/ws/route/source_control.rs "ClientMessage::CommitAndPush { message, .. }"
check_contains apps/cli/src/server/ws/route/source_control.rs "source_control::handle_commit_and_push"
check_contains apps/web/src/components/sidebar/source_control/commit_actions.rs "on_commit_and_push.run(())"
check_contains apps/web/src/i18n/source_control.rs "Commit & Push"

# CommitAndPush is a publish entry point that currently completes as CommitAck,
# not a separate user-visible SyncPush result flow.
check_contains docs/features/operations/sc_commit_and_push.md "CommitAck"
check_contains apps/cli/src/server/handlers/source_control/commits.rs "Commit & Push"
check_absent apps/cli/src/server/handlers/source_control/commits.rs "SyncPush"

echo "source-control-baseline-check: ok"

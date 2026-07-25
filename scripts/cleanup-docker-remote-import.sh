#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=scripts/lib/remote-browser-fixture.sh
source "$ROOT_DIR/scripts/lib/remote-browser-fixture.sh"
# shellcheck source=scripts/lib/docker-remote-import-fixture.sh
source "$ROOT_DIR/scripts/lib/docker-remote-import-fixture.sh"
# shellcheck source=scripts/lib/docker-remote-import-chrome-checkpoint.sh
source "$ROOT_DIR/scripts/lib/docker-remote-import-chrome-checkpoint.sh"

STATE_FILE="$(remote_import_fixture_state_file)"
remote_import_chrome_checkpoint_cleanup "$(dirname -- "$STATE_FILE")"
remote_import_fixture_cleanup "$STATE_FILE"

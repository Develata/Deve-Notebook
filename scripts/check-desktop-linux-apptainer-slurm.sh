#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LABEL="desktop-linux-apptainer-slurm-check"
IMAGE="${DEVE_APPTAINER_IMAGE:-$HOME/.cache/apptainer-images/tauri-build-amd64-7c7dde35be6c.sif}"
IMAGE_SHA256="${DEVE_APPTAINER_IMAGE_SHA256:-d2d5d0f6b999e59728632526c5d64cf111cc9df40b7b4e451feff8cba4242687}"
TOOLCHAIN="${DEVE_APPTAINER_RUST_TOOLCHAIN:-$HOME/.rustup/toolchains/1.97.0-x86_64-unknown-linux-gnu}"
NODE_VERSION="${DEVE_APPTAINER_NODE_VERSION:-v24.18.0}"
WORK_ROOT="${DEVE_APPTAINER_WORK_ROOT:-/tmp}"
WORK="$WORK_ROOT/deve-tauri-exact-${SLURM_JOB_ID:-manual}"
SOURCE_ARCHIVE="${DEVE_APPTAINER_SOURCE_ARCHIVE:-}"
SOURCE_REVISION="${DEVE_APPTAINER_SOURCE_REVISION:-}"
SOURCE_SHA256="${DEVE_APPTAINER_SOURCE_SHA256:-}"
APPTAINER_SCRATCH="/tmp/deve-desktop-apptainer-${SLURM_JOB_ID:-manual}"

fail() {
  echo "$LABEL: $*" >&2
  exit 1
}

sha256_of() {
  sha256sum "$1" | awk '{ print $1 }'
}

require_sha256() {
  local name="$1"
  local value="$2"
  [[ "$value" =~ ^[0-9a-f]{64}$ ]] || fail "$name must be a lowercase SHA-256 digest"
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "required command is missing: $1"
}

load_apptainer() {
  command -v apptainer >/dev/null 2>&1 && return
  command -v module >/dev/null 2>&1 || fail "apptainer is missing and Environment Modules is unavailable"
  module load apptainer/1.4.5
  require_command apptainer
}

load_user_tools() {
  [[ -s "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
  export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
  [[ -s "$NVM_DIR/nvm.sh" ]] || fail "NVM is missing: $NVM_DIR/nvm.sh"
  source "$NVM_DIR/nvm.sh"
  nvm use --silent "$NODE_VERSION" >/dev/null

  [[ "$(node --version)" == "$NODE_VERSION" ]] \
    || fail "Node version mismatch: expected $NODE_VERSION, got $(node --version)"
  [[ "$(trunk --version)" == "trunk 0.21.14" ]] \
    || fail "Trunk 0.21.14 is required"
  [[ "$(wasm-bindgen --version)" == "wasm-bindgen 0.2.121" ]] \
    || fail "wasm-bindgen 0.2.121 is required"
  [[ "$(cargo tauri --version)" == "tauri-cli 2.11.1" ]] \
    || fail "Tauri CLI 2.11.1 is required"
}

safe_extract_archive() {
  local archive="$1"
  local destination="$2"
  python3 - "$archive" "$destination" <<'PY'
import pathlib
import shutil
import sys
import tarfile
archive = pathlib.Path(sys.argv[1])
root = pathlib.Path(sys.argv[2]).resolve()
max_entries = 200_000
max_bytes = 2 * 1024 * 1024 * 1024
seen: set[tuple[str, ...]] = set()
validated: list[tuple[tarfile.TarInfo, tuple[str, ...]]] = []
total_bytes = 0
root.mkdir(parents=True, exist_ok=True)
with tarfile.open(archive, mode="r:gz") as source:
    members = source.getmembers()
    if len(members) > max_entries:
        raise SystemExit("source archive contains too many entries")
    for member in members:
        raw = pathlib.PurePosixPath(member.name)
        if raw.is_absolute() or ".." in raw.parts:
            raise SystemExit(f"unsafe source archive path: {member.name!r}")
        parts = tuple(part for part in raw.parts if part not in ("", "."))
        if not parts:
            if member.isdir():
                continue
            raise SystemExit(f"unsafe empty source archive path: {member.name!r}")
        if parts in seen:
            raise SystemExit(f"duplicate source archive path: {member.name!r}")
        seen.add(parts)
        if not (member.isdir() or member.isfile()):
            raise SystemExit(f"unsupported source archive entry type: {member.name!r}")
        total_bytes += member.size
        if total_bytes > max_bytes:
            raise SystemExit("source archive expands beyond 2 GiB")
        validated.append((member, parts))
    for member, parts in validated:
        if member.isdir():
            root.joinpath(*parts).mkdir(parents=True, exist_ok=True)
    for member, parts in validated:
        if member.isdir():
            continue
        target = root.joinpath(*parts)
        target.parent.mkdir(parents=True, exist_ok=True)
        extracted = source.extractfile(member)
        if extracted is None:
            raise SystemExit(f"cannot read source archive file: {member.name!r}")
        with extracted, target.open("xb") as output:
            shutil.copyfileobj(extracted, output)
        target.chmod(member.mode & 0o777)
PY
}

stage_source() {
  local staged_archive="$WORK/source.tar.gz"

  if [[ -n "$SOURCE_ARCHIVE" ]]; then
    [[ -f "$SOURCE_ARCHIVE" ]] || fail "source archive is missing: $SOURCE_ARCHIVE"
    [[ -n "$SOURCE_REVISION" ]] || fail "DEVE_APPTAINER_SOURCE_REVISION is required with a source archive"
    require_sha256 DEVE_APPTAINER_SOURCE_SHA256 "$SOURCE_SHA256"
    [[ "$(sha256_of "$SOURCE_ARCHIVE")" == "$SOURCE_SHA256" ]] \
      || fail "source archive checksum mismatch"
    cp "$SOURCE_ARCHIVE" "$staged_archive"
  else
    require_command git
    [[ -d "$ROOT_DIR/.git" ]] || fail "default source mode requires a Git worktree"
    [[ -z "$(git -C "$ROOT_DIR" status --porcelain --untracked-files=all)" ]] \
      || fail "default source mode requires a clean worktree"
    SOURCE_REVISION="$(git -C "$ROOT_DIR" rev-parse HEAD)"
    git -C "$ROOT_DIR" archive --format=tar.gz --output="$staged_archive" HEAD
    SOURCE_SHA256="$(sha256_of "$staged_archive")"
  fi

  safe_extract_archive "$staged_archive" "$WORK/repo"
  echo "SOURCE_REVISION=$SOURCE_REVISION"
  echo "SOURCE_SHA256=$SOURCE_SHA256"
}

container_exec() {
  apptainer exec --cleanenv \
    --env "PATH=$CONTAINER_PATH" \
    --env "CARGO_HOME=$CARGO_HOME" \
    --env "CARGO_TARGET_DIR=$CARGO_TARGET_DIR" \
    --env "CARGO_BIN=$CARGO_BIN" \
    --env "RUSTC=$RUSTC" \
    --env "RUSTDOC=$RUSTDOC" \
    --env "CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS" \
    --env "CARGO_INCREMENTAL=0" \
    --env "CARGO_NET_OFFLINE=true" \
    "$IMAGE" "$@"
}

[[ -n "${SLURM_JOB_ID:-}" ]] || fail "run this worker inside a Slurm allocation"
[[ -d "$WORK_ROOT" && -w "$WORK_ROOT" ]] || fail "work root is not writable: $WORK_ROOT"
require_command sha256sum
require_command tar
require_command cp
require_command python3
load_apptainer
load_user_tools
require_sha256 DEVE_APPTAINER_IMAGE_SHA256 "$IMAGE_SHA256"
[[ -s "$IMAGE" ]] || fail "Apptainer image is missing: $IMAGE"
[[ "$(sha256_of "$IMAGE")" == "$IMAGE_SHA256" ]] || fail "Apptainer image checksum mismatch"
[[ -d "$TOOLCHAIN" ]] || fail "Rust toolchain is missing: $TOOLCHAIN"

rm -rf "$WORK"
mkdir -p "$WORK/toolchain"
cleanup() {
  rm -rf "$WORK" "$APPTAINER_SCRATCH"
}
trap cleanup EXIT

echo "HOST=$(hostname)"
echo "SLURM_JOB_ID=$SLURM_JOB_ID"
echo "IMAGE_SHA256=$IMAGE_SHA256"
echo "NODE=$(node --version)"
echo "TAURI=$(cargo tauri --version)"

echo "GATE=stage_exact_source"
stage_source

echo "GATE=stage_node_local_toolchain"
cp -a "$TOOLCHAIN/." "$WORK/toolchain/"

echo "GATE=prepare_apptainer_tmpdir"
export APPTAINER_TMPDIR="$APPTAINER_SCRATCH"
rm -rf "$APPTAINER_SCRATCH"
mkdir -p "$APPTAINER_SCRATCH"

export CARGO_HOME="$WORK/.cargo-home"
export WEB_CARGO_TARGET_DIR="$WORK/web-target"
export CARGO_TARGET_DIR="$WEB_CARGO_TARGET_DIR"
export CARGO_BIN="$WORK/toolchain/bin/cargo"
export RUSTC="$WORK/toolchain/bin/rustc"
export RUSTDOC="$WORK/toolchain/bin/rustdoc"
export CARGO_BUILD_JOBS="${DEVE_APPTAINER_CARGO_BUILD_JOBS:-1}"
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
NODE_BIN="$HOME/.nvm/versions/node/$NODE_VERSION/bin"
export PATH="$WORK/toolchain/bin:$HOME/.cargo/bin:$HOME/.local/bin:$NODE_BIN:$PATH"
mkdir -p "$CARGO_HOME/registry" "$CARGO_TARGET_DIR"
tar -C "$HOME/.cargo/registry" --exclude='./src' -cf - . \
  | tar -C "$CARGO_HOME/registry" -xf -

cd "$WORK/repo/apps/web"
echo "GATE=npm_ci"
npm ci --no-audit --no-fund
cd "$WORK/repo"
echo "GATE=web_release_build"
scripts/smoke-web-release-build.sh

export CARGO_TARGET_DIR="$WORK/repo/target"
mkdir -p "$CARGO_TARGET_DIR"
CONTAINER_PATH="$WORK/toolchain/bin:$HOME/.cargo/bin:$HOME/.local/bin:$NODE_BIN:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

echo "GATE=desktop_native_container_session"
container_exec bash -c '
  set -euo pipefail
  cd "${CARGO_TARGET_DIR%/target}"

  echo "GATE=container_sysdeps"
  for p in gtk+-3.0 webkit2gtk-4.1 javascriptcoregtk-4.1 ayatana-appindicator3-0.1 librsvg-2.0 openssl; do
    printf "%s=" "$p"
    pkg-config --modversion "$p"
  done
  command -v dpkg-deb

  echo "GATE=desktop_package_build"
  DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 \
    DEVE_DESKTOP_PACKAGE_BUNDLES=deb \
    DEVE_DESKTOP_PACKAGE_NO_SIGN=1 \
    scripts/check-desktop-platform-package-build.sh

  echo "GATE=desktop_package_startup_smoke"
  DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 \
    DEVE_DESKTOP_PACKAGE_BUNDLES=deb \
    scripts/check-desktop-package-startup-smoke.sh

  echo "GATE=desktop_native_session_package_smoke"
  DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED=1 \
    DEVE_DESKTOP_PACKAGE_BUNDLES=deb \
    scripts/check-desktop-native-session-package-smoke.sh

  DEB="$(find "$CARGO_TARGET_DIR/release/bundle/deb" -maxdepth 1 -type f -name "*.deb" -print -quit)"
  [[ -s "$DEB" ]] || { echo "desktop-linux-apptainer-slurm-check: Debian package output is missing" >&2; exit 1; }
  dpkg-deb --info "$DEB" | sed -n "1,100p"
  stat -c "DEB_BYTES=%s" "$DEB"
  echo "DESKTOP_LINUX_APPTAINER_SLURM_OK=1"
'

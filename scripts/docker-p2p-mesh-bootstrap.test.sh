#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf -- "$fixture"' EXIT
fake_bin="$fixture/bin"
mkdir -p "$fake_bin"
cat >"$fake_bin/deve_cli" <<'EOF'
#!/usr/bin/env sh
test "$1" = "serve"
printf 'served\n' >"$BOOTSTRAP_SERVED"
EOF
chmod +x "$fake_bin/deve_cli"

repo_id="11111111-1111-1111-1111-111111111111"
repo_key="deve_mesh_shared_repo_key_32!!!!"

write_record() {
  local output="$1"
  local id="$2"
  local segment="$3"
  local base="$4"
  cat >>"$output" <<EOF
[[locators]]
repo_id = '$id'
workspace_segment = '$segment'
projection_base_abs = '$base'
EOF
}

prepare_workspace() {
  local base="$1"
  local segment="$2"
  mkdir -p "$base/$segment/.notegit/keys"
  printf "version = 1\nrepo_id = '%s'\nrepo_name = 'default'\n" \
    "$repo_id" >"$base/$segment/.notegit/identity.toml"
  printf '%s' "$repo_key" >"$base/$segment/.notegit/keys/repo.key"
}

run_bootstrap() {
  local locator="$1"
  local base="$2"
  BOOTSTRAP_SERVED="$fixture/served" \
  DEVE_DOCKER_P2P_MESH_REPO_ID="$repo_id" \
  DEVE_DOCKER_P2P_MESH_REPO_KEY="$repo_key" \
  DEVE_DOCKER_P2P_MESH_LOCATOR_FILE="$locator" \
  DEVE_DOCKER_P2P_MESH_EXPECTED_PROJECTION_BASE="$base" \
  PATH="$fake_bin:$PATH" \
    sh "$ROOT_DIR/scripts/docker-p2p-mesh-bootstrap.sh"
}

success="$fixture/success"
mkdir -p "$success"
success_locator="$success/locators.toml"
write_record "$success_locator" "$repo_id" "$repo_id" "$success/notes"
prepare_workspace "$success/notes" "$repo_id"
run_bootstrap "$success_locator" "$success/notes"
[[ -f "$fixture/served" ]]
[[ "$(cat "$success/notes/$repo_id/.notegit/keys/repo.key")" == "$repo_key" ]]
key_mode="$(stat -c '%a' "$success/notes/$repo_id/.notegit/keys/repo.key")"
if [[ "$key_mode" != "600" ]]; then
  case "$(uname -s)" in
    MINGW*|MSYS*)
      echo "docker-p2p-mesh-bootstrap-test: mode case skipped on Windows ACL filesystem"
      ;;
    *)
      echo "repo key mode is $key_mode, expected 600" >&2
      exit 1
      ;;
  esac
fi
if find "$success/notes/$repo_id/.notegit/keys" -name '.repo.key.tmp.*' -print -quit \
    | grep -q .; then
  echo "successful bootstrap left a temporary key behind" >&2
  exit 1
fi
rm -f "$fixture/served"

cross="$fixture/cross-record"
mkdir -p "$cross"
cross_locator="$cross/locators.toml"
write_record "$cross_locator" "$repo_id" "$repo_id" "$cross/outside"
write_record "$cross_locator" "22222222-2222-2222-2222-222222222222" \
  "22222222-2222-2222-2222-222222222222" "$cross/notes"
prepare_workspace "$cross/outside" "$repo_id"
if run_bootstrap "$cross_locator" "$cross/notes"; then
  echo "cross-record projection base was accepted" >&2
  exit 1
fi

for segment in . ..; do
  dot="$fixture/dot-${segment//./x}"
  mkdir -p "$dot/notes/.notegit/keys"
  dot_locator="$dot/locators.toml"
  write_record "$dot_locator" "$repo_id" "$segment" "$dot/notes"
  if run_bootstrap "$dot_locator" "$dot/notes"; then
    echo "dot workspace segment was accepted: $segment" >&2
    exit 1
  fi
done

duplicate="$fixture/duplicate"
mkdir -p "$duplicate"
duplicate_locator="$duplicate/locators.toml"
write_record "$duplicate_locator" "$repo_id" "$repo_id" "$duplicate/notes"
write_record "$duplicate_locator" "$repo_id" "alias--$repo_id" "$duplicate/notes"
prepare_workspace "$duplicate/notes" "$repo_id"
prepare_workspace "$duplicate/notes" "alias--$repo_id"
if run_bootstrap "$duplicate_locator" "$duplicate/notes"; then
  echo "duplicate RepoId locator was accepted" >&2
  exit 1
fi

identity="$fixture/identity-mismatch"
mkdir -p "$identity"
identity_locator="$identity/locators.toml"
write_record "$identity_locator" "$repo_id" "$repo_id" "$identity/notes"
prepare_workspace "$identity/notes" "$repo_id"
sed -i "s/$repo_id/22222222-2222-2222-2222-222222222222/" \
  "$identity/notes/$repo_id/.notegit/identity.toml"
if run_bootstrap "$identity_locator" "$identity/notes"; then
  echo "workspace identity mismatch was accepted" >&2
  exit 1
fi

key_mismatch="$fixture/key-mismatch"
mkdir -p "$key_mismatch"
key_mismatch_locator="$key_mismatch/locators.toml"
write_record "$key_mismatch_locator" "$repo_id" "$repo_id" "$key_mismatch/notes"
prepare_workspace "$key_mismatch/notes" "$repo_id"
printf 'different-existing-key-material!!' \
  >"$key_mismatch/notes/$repo_id/.notegit/keys/repo.key"
if run_bootstrap "$key_mismatch_locator" "$key_mismatch/notes"; then
  echo "existing RepoKey mismatch was accepted" >&2
  exit 1
fi
[[ "$(cat "$key_mismatch/notes/$repo_id/.notegit/keys/repo.key")" \
  == "different-existing-key-material!!" ]]

attack="$fixture/temp-symlink"
mkdir -p "$attack/bin"
attack_locator="$attack/locators.toml"
write_record "$attack_locator" "$repo_id" "$repo_id" "$attack/notes"
prepare_workspace "$attack/notes" "$repo_id"
rm -f -- "$attack/notes/$repo_id/.notegit/keys/repo.key"
attack_target="$attack/target"
printf 'preserve-me' >"$attack_target"
if ln -s "$attack_target" "$attack/fake-tmp" 2>/dev/null \
    && [[ -L "$attack/fake-tmp" ]]; then
  cat >"$attack/bin/mktemp" <<EOF
#!/usr/bin/env sh
printf '%s\n' '$attack/fake-tmp'
EOF
  chmod +x "$attack/bin/mktemp"
  if BOOTSTRAP_SERVED="$fixture/served" \
      DEVE_DOCKER_P2P_MESH_REPO_ID="$repo_id" \
      DEVE_DOCKER_P2P_MESH_REPO_KEY="$repo_key" \
      DEVE_DOCKER_P2P_MESH_LOCATOR_FILE="$attack_locator" \
      DEVE_DOCKER_P2P_MESH_EXPECTED_PROJECTION_BASE="$attack/notes" \
      PATH="$attack/bin:$fake_bin:$PATH" \
        sh "$ROOT_DIR/scripts/docker-p2p-mesh-bootstrap.sh"; then
    echo "symlink temporary key was accepted" >&2
    exit 1
  fi
  [[ "$(cat "$attack_target")" == "preserve-me" ]]
else
  rm -f -- "$attack/fake-tmp"
  echo "docker-p2p-mesh-bootstrap-test: symlink case skipped on this filesystem"
fi

term="$fixture/term"
mkdir -p "$term/bin"
term_locator="$term/locators.toml"
write_record "$term_locator" "$repo_id" "$repo_id" "$term/notes"
prepare_workspace "$term/notes" "$repo_id"
rm -f -- "$term/notes/$repo_id/.notegit/keys/repo.key"
cat >"$term/bin/mv" <<'EOF'
#!/usr/bin/env sh
kill -TERM "$PPID"
sleep 1
exit 0
EOF
chmod +x "$term/bin/mv"
if BOOTSTRAP_SERVED="$fixture/term-served" \
    DEVE_DOCKER_P2P_MESH_REPO_ID="$repo_id" \
    DEVE_DOCKER_P2P_MESH_REPO_KEY="$repo_key" \
    DEVE_DOCKER_P2P_MESH_LOCATOR_FILE="$term_locator" \
    DEVE_DOCKER_P2P_MESH_EXPECTED_PROJECTION_BASE="$term/notes" \
    PATH="$term/bin:$fake_bin:$PATH" \
      sh "$ROOT_DIR/scripts/docker-p2p-mesh-bootstrap.sh"; then
  echo "TERM during key publication was swallowed" >&2
  exit 1
fi
[[ ! -e "$fixture/term-served" ]]
if find "$term/notes/$repo_id/.notegit/keys" -name '.repo.key.tmp.*' -print -quit \
    | grep -q .; then
  echo "TERM left a temporary key behind" >&2
  exit 1
fi

echo "docker-p2p-mesh-bootstrap-test: ok"

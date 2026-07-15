#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$ROOT_DIR/scripts/dispatch-native-target-host-workflow.sh"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TMP_DIR"' EXIT

fail() {
  echo "dispatch-native-target-host-workflow-test: $*" >&2
  exit 1
}

run_clean() {
  env \
    -u DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_REMOTE_BROWSER_SMOKE \
    -u DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HTTPS_ORIGIN \
    -u DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_USERNAME \
    -u DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HEAD_PROOF_URL \
    -u DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE \
    -u DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN \
    -u DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_USERNAME \
    -u DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_HEAD_PROOF_URL \
    -u DEVE_GITHUB_TOKEN \
    -u GH_TOKEN \
    -u GITHUB_TOKEN \
    DEVE_NATIVE_TARGET_HOST_REF=main \
    DEVE_NATIVE_TARGET_HOST_REPOSITORY=owner/repo \
    "$@" \
    bash "$SCRIPT"
}

assert_contains() {
  local path="$1"
  local expected="$2"
  grep -Fqx -- "$expected" "$path" || fail "missing captured argument: $expected"
}

assert_fails() {
  local expected="$1"
  shift
  local output

  if output="$(run_clean "$@" 2>&1)"; then
    fail "expected failure containing: $expected"
  fi
  [[ "$output" == *"$expected"* ]] || fail "unexpected failure; wanted '$expected', got: $output"
}

cat >"$TMP_DIR/gh-ok" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "auth" && "${2:-}" == "status" ]]; then
  exit 0
fi
printf '%s\n' "$@" >"$DEVE_TEST_GH_CAPTURE"
SH
chmod +x "$TMP_DIR/gh-ok"

GH_CAPTURE="$TMP_DIR/gh-args.txt"
run_clean \
  DEVE_NATIVE_TARGET_HOST_DISPATCH=1 \
  DEVE_GH_BIN="$TMP_DIR/gh-ok" \
  DEVE_TEST_GH_CAPTURE="$GH_CAPTURE" \
  DEVE_NATIVE_TARGET_HOST_TARGET=all \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_INSTALLER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_REMOTE_BROWSER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HTTPS_ORIGIN=https://desktop.example.test \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_USERNAME=desktop-user \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HEAD_PROOF_URL=https://desktop.example.test/.well-known/deve-head \
  DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_PACKAGE_BUILD=true \
  DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN=https://android.example.test:8443 \
  DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_USERNAME=android-user \
  DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_HEAD_PROOF_URL=https://android.example.test:8443/.well-known/deve-head \
  >/dev/null

assert_contains "$GH_CAPTURE" "run_desktop_remote_browser_smoke=true"
assert_contains "$GH_CAPTURE" "desktop_remote_https_origin=https://desktop.example.test"
assert_contains "$GH_CAPTURE" "desktop_remote_username=desktop-user"
assert_contains "$GH_CAPTURE" "desktop_remote_head_proof_url=https://desktop.example.test/.well-known/deve-head"
assert_contains "$GH_CAPTURE" "run_mobile_android_remote_browser_smoke=true"
assert_contains "$GH_CAPTURE" "mobile_android_remote_https_origin=https://android.example.test:8443"
assert_contains "$GH_CAPTURE" "mobile_android_remote_username=android-user"
assert_contains "$GH_CAPTURE" "mobile_android_remote_head_proof_url=https://android.example.test:8443/.well-known/deve-head"
if grep -Eqi 'password|auth_secret' "$GH_CAPTURE"; then
  fail "dispatch command unexpectedly contains a password/auth-secret input"
fi

cat >"$TMP_DIR/gh-no-auth" <<'SH'
#!/usr/bin/env bash
exit 1
SH
cat >"$TMP_DIR/curl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
while (($#)); do
  if [[ "$1" == "--data-binary" ]]; then
    printf '%s' "$2" >"$DEVE_TEST_API_PAYLOAD"
    exit 0
  fi
  shift
done
exit 2
SH
chmod +x "$TMP_DIR/gh-no-auth" "$TMP_DIR/curl"

API_PAYLOAD="$TMP_DIR/api-payload.json"
run_clean \
  PATH="$TMP_DIR:$PATH" \
  DEVE_NATIVE_TARGET_HOST_DISPATCH=1 \
  DEVE_GH_BIN="$TMP_DIR/gh-no-auth" \
  DEVE_GITHUB_TOKEN=test-token \
  DEVE_TEST_API_PAYLOAD="$API_PAYLOAD" \
  DEVE_NATIVE_TARGET_HOST_TARGET=all \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_INSTALLER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_REMOTE_BROWSER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HTTPS_ORIGIN=https://desktop.example.test \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_USERNAME=desktop-user \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HEAD_PROOF_URL=https://desktop.example.test/head \
  DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_PACKAGE_BUILD=true \
  DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN=https://android.example.test \
  DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_USERNAME=android-user \
  DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_HEAD_PROOF_URL=https://android.example.test/head \
  >/dev/null

grep -Fq '"run_desktop_remote_browser_smoke":"true"' "$API_PAYLOAD" || fail "API payload lacks Desktop RemoteBrowser boolean"
grep -Fq '"desktop_remote_https_origin":"https://desktop.example.test"' "$API_PAYLOAD" || fail "API payload lacks Desktop origin"
grep -Fq '"desktop_remote_username":"desktop-user"' "$API_PAYLOAD" || fail "API payload lacks Desktop username"
grep -Fq '"desktop_remote_head_proof_url":"https://desktop.example.test/head"' "$API_PAYLOAD" || fail "API payload lacks Desktop HEAD proof URL"
grep -Fq '"run_mobile_android_remote_browser_smoke":"true"' "$API_PAYLOAD" || fail "API payload lacks Android RemoteBrowser boolean"
grep -Fq '"mobile_android_remote_https_origin":"https://android.example.test"' "$API_PAYLOAD" || fail "API payload lacks Android origin"
grep -Fq '"mobile_android_remote_username":"android-user"' "$API_PAYLOAD" || fail "API payload lacks Android username"
grep -Fq '"mobile_android_remote_head_proof_url":"https://android.example.test/head"' "$API_PAYLOAD" || fail "API payload lacks Android HEAD proof URL"
if grep -Eqi 'password|auth_secret|test-token' "$API_PAYLOAD"; then
  fail "API payload unexpectedly contains a password, auth secret, or dispatch token"
fi

assert_fails \
  "desktop RemoteBrowser smoke requires desktop package build and installer smoke" \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_REMOTE_BROWSER_SMOKE=true
assert_fails \
  "Android RemoteBrowser smoke requires Android package build and install/startup smoke" \
  DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE=true
assert_fails \
  "desktop RemoteBrowser smoke requires target=all or target=desktop-windows" \
  DEVE_NATIVE_TARGET_HOST_TARGET=mobile-android \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_INSTALLER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_REMOTE_BROWSER_SMOKE=true
assert_fails \
  "desktop RemoteBrowser external override requires HTTPS origin, username, and same-origin HEAD proof URL together" \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_INSTALLER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_REMOTE_BROWSER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HTTPS_ORIGIN=https://desktop.example.test
assert_fails \
  "Android RemoteBrowser external override requires HTTPS origin, username, and same-origin HEAD proof URL together" \
  DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_PACKAGE_BUILD=true \
  DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_MOBILE_ANDROID_REMOTE_USERNAME=android-user
assert_fails \
  "desktop RemoteBrowser external override requires its RemoteBrowser smoke to be enabled" \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HTTPS_ORIGIN=https://desktop.example.test \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_USERNAME=desktop-user \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HEAD_PROOF_URL=https://desktop.example.test/head
assert_fails \
  "desktop RemoteBrowser HEAD proof URL must use the RemoteBrowser HTTPS origin" \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_INSTALLER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_REMOTE_BROWSER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HTTPS_ORIGIN=https://desktop.example.test \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_USERNAME=desktop-user \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HEAD_PROOF_URL=https://other.example.test/head
assert_fails \
  "desktop RemoteBrowser username contains a forbidden CR/LF control character" \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_INSTALLER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_REMOTE_BROWSER_SMOKE=true \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HTTPS_ORIGIN=https://desktop.example.test \
  $'DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_USERNAME=user\r\ninjected' \
  DEVE_NATIVE_TARGET_HOST_DESKTOP_REMOTE_HEAD_PROOF_URL=https://desktop.example.test/head
assert_fails \
  "Git ref contains a forbidden CR/LF control character" \
  $'DEVE_NATIVE_TARGET_HOST_REF=main\ninvalid'
assert_fails \
  "GitHub token contains a forbidden CR/LF control character" \
  $'DEVE_GITHUB_TOKEN=token\rinjected'

echo "dispatch-native-target-host-workflow-test: ok"

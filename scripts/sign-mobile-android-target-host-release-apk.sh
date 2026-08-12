#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/android-tools.sh"

REQUIRED="${DEVE_MOBILE_ANDROID_TARGET_HOST_RELEASE_SIGN_REQUIRED:-0}"
BUILD_TOOLS_VERSION="${DEVE_MOBILE_ANDROID_BUILD_TOOLS_VERSION:-36.0.0}"
UNSIGNED_APK="${DEVE_MOBILE_ANDROID_UNSIGNED_APK_PATH:-apps/mobile/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk}"
SIGNED_APK="${DEVE_MOBILE_ANDROID_SIGNED_APK_PATH:-target/mobile-android-emulator-smoke/app-universal-release-target-host-test-signed.apk}"
DIAGNOSTIC_ROOT="${DEVE_MOBILE_ANDROID_DIAGNOSTIC_ROOT:-target/mobile-android-emulator-smoke}"

# This signer exists only to make a minified release-variant APK installable on
# an owned target-host emulator. It never consumes release signing material and
# its output is not a candidate artifact.

fail() {
  echo "mobile-android-target-host-release-sign: $*" >&2
  exit 1
}

resolve_repo_path() {
  local path="$1"
  if [[ "$path" = /* || "$path" =~ ^[A-Za-z]:[/\\] ]]; then
    printf '%s\n' "$path"
  else
    printf '%s\n' "$ROOT_DIR/$path"
  fi
}

if [[ "$REQUIRED" != "1" ]]; then
  echo "mobile-android-target-host-release-sign: signing not executed; set DEVE_MOBILE_ANDROID_TARGET_HOST_RELEASE_SIGN_REQUIRED=1 on an Android target host"
  echo "mobile-android-target-host-release-sign: ok"
  exit 0
fi

android_prepare_java_home || fail "Java 17+ or Android Studio JBR is required"
command -v java >/dev/null 2>&1 || fail "java is required"
command -v keytool >/dev/null 2>&1 || fail "keytool is required"

sdk="$(android_sdk_root)" || fail "Android SDK root is unavailable"
build_tools="$sdk/build-tools/$BUILD_TOOLS_VERSION"
apksigner_jar="$build_tools/lib/apksigner.jar"
zipalign="$build_tools/zipalign"
[[ -x "$zipalign" ]] || zipalign="$build_tools/zipalign.exe"
[[ -f "$apksigner_jar" ]] || fail "apksigner.jar is unavailable in build-tools $BUILD_TOOLS_VERSION"
[[ -x "$zipalign" ]] || fail "zipalign is unavailable in build-tools $BUILD_TOOLS_VERSION"

unsigned_apk="$(resolve_repo_path "$UNSIGNED_APK")"
signed_apk="$(resolve_repo_path "$SIGNED_APK")"
diagnostic_root="$(resolve_repo_path "$DIAGNOSTIC_ROOT")"
[[ -f "$unsigned_apk" ]] || fail "unsigned release APK is missing: $UNSIGNED_APK"
[[ "$unsigned_apk" != "$signed_apk" ]] || fail "signed output must differ from unsigned input"
repo_target="$ROOT_DIR/target"
[[ -d "$repo_target" ]] || fail "repository target directory is unavailable"
[[ -d "$diagnostic_root" ]] || fail "owned diagnostic root must already exist"
[[ -d "$(dirname "$signed_apk")" ]] || fail "signed output parent must already exist"
repo_target="$(cd "$repo_target" && pwd -P)"
diagnostic_root="$(cd "$diagnostic_root" && pwd -P)"
signed_parent="$(cd "$(dirname "$signed_apk")" && pwd -P)"
signed_apk="$signed_parent/$(basename "$signed_apk")"

case "$diagnostic_root/" in
  "$repo_target/"*) ;;
  *) fail "diagnostic root must stay within the repository target directory" ;;
esac
case "$signed_apk" in
  "$diagnostic_root"/*) ;;
  *) fail "signed output must stay within the owned diagnostic root" ;;
esac
[[ ! -e "$signed_apk" && ! -e "${signed_apk}.idsig" ]] \
  || fail "diagnostic signed output already exists"

temp_dir="$(mktemp -d "$diagnostic_root/.deve-android-target-host-sign.XXXXXX")"
keystore="$temp_dir/diagnostic.keystore"
aligned_apk="$temp_dir/aligned.apk"
temporary_signed_apk="$temp_dir/signed.apk"
signing_complete=0
cleanup() {
  rm -f -- "$keystore" "$aligned_apk" "$temporary_signed_apk" "${temporary_signed_apk}.idsig"
  if [[ "$signing_complete" != "1" ]]; then
    rm -f -- "$signed_apk" "${signed_apk}.idsig"
  fi
  rmdir "$temp_dir" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Fixed credentials are intentionally non-secret: the keystore is ephemeral,
# deleted before return, and must never be accepted as a release signer.
keytool -genkeypair -noprompt \
  -keystore "$keystore" \
  -storepass android \
  -keypass android \
  -alias deve-target-host \
  -keyalg RSA \
  -keysize 2048 \
  -validity 2 \
  -dname "CN=Deve Target Host Diagnostic,OU=CI,O=Deve,L=Local,ST=Local,C=US" \
  >/dev/null 2>&1

"$zipalign" -f 4 "$unsigned_apk" "$aligned_apk"
java -jar "$apksigner_jar" sign \
  --ks "$keystore" \
  --ks-pass pass:android \
  --ks-key-alias deve-target-host \
  --key-pass pass:android \
  --out "$temporary_signed_apk" \
  "$aligned_apk"
java -jar "$apksigner_jar" verify --verbose "$temporary_signed_apk" >/dev/null
mv -- "$temporary_signed_apk" "$signed_apk"
rm -f -- "${temporary_signed_apk}.idsig"
signing_complete=1

echo "mobile-android-target-host-release-sign: output=$SIGNED_APK signer=ephemeral-diagnostic"
echo "mobile-android-target-host-release-sign: ok"

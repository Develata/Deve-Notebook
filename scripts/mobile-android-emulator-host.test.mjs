import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const read = (path) => fs.readFileSync(new URL(path, import.meta.url), "utf8");
const orchestrator = read("./check-mobile-android-emulator-install-startup-smoke.sh");
const emulatorPin = read("./lib/android-emulator-pin.sh");
const emulatorPinTest = read("./android-emulator-pin.test.sh");
const emulatorRenderer = read("./lib/android-emulator-renderer.sh");
const emulatorRendererTest = read("./android-emulator-renderer.test.sh");
const emulatorFeaturePolicy = read("./lib/android-emulator-feature-policy.sh");
const releaseNativeWorkflow = read("../.github/workflows/release-native.yml");
const nativeTargetHostWorkflow = read("../.github/workflows/native-target-host.yml");
const emulatorHostPreparation = read("./prepare-android-emulator-host.sh");
const producerRegistry = JSON.parse(read("../docs/registry/acceptance-producers.json"));

test("emulator gate pins the exact stable emulator build fail-closed", () => {
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_VERSION="36\.6\.11\.0"/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_BUILD_ID="15507667"/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_SHA256_LINUX="[0-9a-f]{64}"/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_SHA256_WINDOWS="[0-9a-f]{64}"/);
  assert.match(emulatorPin, /sha256sum -c --quiet -/);
  assert.match(
    emulatorPin,
    /sha256sum[\s\S]*unzip[\s\S]*android_emulator_pin_publish_extracted "\$cache_root" "\$staging" "\$extracted"/,
  );
  assert.match(emulatorPin, /\.publish-\$ANDROID_EMULATOR_PIN_BUILD_ID\.lock/);
  assert.match(emulatorPin, /android_emulator_pin_acquire_publish_lock[\s\S]*ln -- "\$owner_file" "\$lock_file"/);
  assert.match(
    emulatorPin,
    /android_emulator_pin_acquire_publish_lock "\$lock_file" "\$owner_file" "\$owner_token"[\s\S]*android_emulator_pin_owns_publish_lock[\s\S]*android_emulator_pin_matches "\$binary"/,
  );
  assert.match(emulatorPinTest, /publisher_pids/);
  assert.match(emulatorPinTest, /acquisition-window signal leaked the build lock/);
  assert.match(emulatorPin, /android_emulator_pin_matches "\$binary"/);
  assert.match(emulatorPin, /does not match pin \$ANDROID_EMULATOR_PIN_VERSION/);
  assert.match(emulatorPin, /downloaded emulator does not match pin/);
  assert.match(emulatorPin, /timeout --signal=TERM --kill-after=5s/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES/);
  assert.match(emulatorPin, /Android emulator version \[0-9\]/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_LAST_PROBE/);
  assert.match(emulatorPinTest, /expect_match nonzero-canonical/);
  assert.match(emulatorPinTest, /expect_reject loose-tokens/);
  assert.match(emulatorPinTest, /expect_reject timeout/);
  assert.match(emulatorPinTest, /expect_reject oversized/);
  assert.match(emulatorPin, /DEVE_MOBILE_ANDROID_EMULATOR_PIN_DIR:-\$HOME\/\.cache\/deve-android-emulator-pin/);
  assert.doesNotMatch(emulatorPin, /android_run_tool|sdkmanager_cmd/);
});

test("Android target-host workflows share the pinned emulator host preparation", () => {
  const mobileAndroidJob = releaseNativeWorkflow.slice(releaseNativeWorkflow.indexOf("  mobile-android:"));
  const nativeAndroidJob = nativeTargetHostWorkflow.slice(
    nativeTargetHostWorkflow.indexOf("  mobile-android:"),
    nativeTargetHostWorkflow.indexOf("\n  mobile-ios:"),
  );
  assert.ok(mobileAndroidJob.startsWith("  mobile-android:"));
  assert.ok(nativeAndroidJob.startsWith("  mobile-android:"));
  assert.match(mobileAndroidJob, /run: bash scripts\/prepare-android-emulator-host\.sh/);
  assert.match(nativeAndroidJob, /if: \$\{\{ inputs\.run_mobile_android_package_build && inputs\.run_mobile_android_install_startup_smoke \}\}\n        run: bash scripts\/prepare-android-emulator-host\.sh/);
  assert.match(emulatorHostPreparation, /set -euo pipefail/);
  assert.match(emulatorHostPreparation, /sudo apt-get update/);
  assert.match(emulatorHostPreparation, /sudo apt-get install -y --no-install-recommends libpulse0/);
  assert.match(emulatorHostPreparation, /ldconfig -p \| grep -F 'libpulse\.so\.0'/);
  assert.match(emulatorHostPreparation, /99-kvm4all\.rules[\s\S]*udevadm trigger --name-match=kvm/);
  const android = producerRegistry.producers.filter(({ producer_id }) =>
    ["android.local-backend", "android.remote-browser"].includes(producer_id));
  assert.ok(android.every(({ artifacts }) => artifacts.includes("scripts/prepare-android-emulator-host.sh")));
});

test("emulator orchestrator launches only the resolved pinned binary", () => {
  assert.match(orchestrator, /source "\$ROOT_DIR\/scripts\/lib\/android-emulator-pin\.sh"/);
  assert.match(orchestrator, /PINNED_EMULATOR_BIN="\$\(android_resolve_pinned_emulator\)"/);
  assert.match(orchestrator, /pinned Android emulator was not resolved before emulator_cmd/);
  assert.match(orchestrator, /pinned emulator: \$PINNED_EMULATOR_BIN/);
  assert.doesNotMatch(orchestrator, /\$PINNED_EMULATOR_BIN" -version/);
  assert.match(orchestrator, /"\$PINNED_EMULATOR_BIN" "\$@"/);
});

test("emulator gate pins the API 37.0 image with swangle and 4096 MiB", () => {
  assert.match(orchestrator, /API_LEVEL="\$\{DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL:-37\.0\}"/);
  assert.match(orchestrator, /EMULATOR_RAM_MB="\$\{DEVE_MOBILE_ANDROID_EMULATOR_RAM_MB:-4096\}"/);
  assert.match(orchestrator, /-gpu swangle/);
  assert.doesNotMatch(orchestrator, /swiftshader_indirect/);
  assert.match(orchestrator, /android_emulator_renderer_verify/);
  assert.match(emulatorRenderer, /ANDROID_EMULATOR_RENDERER_LOG_READ_BYTES/);
  assert.match(emulatorRenderer, /swiftshader_indirect/);
  assert.match(emulatorRenderer, /vulkan_mode_selected/);
  assert.doesNotMatch(emulatorRenderer, /grep -aE -m 16/);
  assert.match(emulatorRendererTest, /conflicting renderer evidence/);
  assert.match(emulatorRendererTest, /for _ in \{1\.\.16\}/);
  assert.match(emulatorRendererTest, /selection beyond bounded log prefix/);
});

test("formal emulator gate requires the observed DMA feature conjunction", () => {
  assert.match(orchestrator, /source "\$ROOT_DIR\/scripts\/lib\/android-emulator-feature-policy\.sh"/);
  assert.match(orchestrator, /FORMAL_FEATURE_POLICY="direct-memory-shared-slots"/);
  assert.match(orchestrator, /"\$\{ANDROID_EMULATOR_FEATURE_ARGS\[@\]\}"/);
  assert.match(orchestrator, /android_emulator_feature_policy_wait[\s\S]*wait_for_boot/);
  assert.match(orchestrator, /ensure_emulator_process_alive\s+android_emulator_feature_policy_observe[\s\S]*ensure_emulator_process_alive\s+echo "mobile-android-emulator-install-startup-smoke-check: serial=/);
  assert.match(emulatorFeaturePolicy, /-feature GLDirectMem[\s\S]*-feature HasSharedSlotsHostMemoryAllocator/);
  assert.match(emulatorFeaturePolicy, /ANDROID_EMULATOR_FEATURE_POLICY_EXPECTED_PAIR="1\/1"/);
});

test("android producers bind emulator pin, renderer, and feature proof", () => {
  for (const producerId of ["android.local-backend", "android.remote-browser"]) {
    const producer = producerRegistry.producers.find(({ producer_id }) => producer_id === producerId);
    assert.ok(producer, `${producerId} must exist in the producer registry`);
    assert.ok(producer.artifacts.includes("scripts/lib/android-emulator-pin.sh"));
    for (const artifact of [
      "scripts/android-emulator-pin.test.sh",
      "scripts/android-emulator-renderer.test.sh",
      "scripts/android-emulator-feature-policy.test.sh",
      "scripts/lib/android-emulator-renderer.sh",
      "scripts/lib/android-emulator-feature-policy.sh",
      "scripts/lib/android-emulator-diagnostics.sh",
    ]) assert.ok(producer.artifacts.includes(artifact), `${producerId} receipt must bind ${artifact}`);
  }
});

test("Android producer receipts bind presentation and lifecycle proof code", () => {
  const producer = (id) => producerRegistry.producers.find(({ producer_id }) => producer_id === id);
  const local = producer("android.local-backend");
  const remote = producer("android.remote-browser");
  const common = [
    "scripts/mobile-android-emulator-host.test.mjs",
    "scripts/mobile-android-presentation.test.mjs",
    "scripts/lib/android-lifecycle-harness.mjs",
    "scripts/android-lifecycle-harness.test.mjs",
    "scripts/lib/android-drawer-touch-proof.mjs",
    "scripts/lib/android-presentation-proof.mjs",
    "scripts/lib/mobile-ime-back-proof.mjs",
    "scripts/lib/mobile-editor-session-observation.mjs",
    "scripts/mobile-editor-session-observation.test.mjs",
    "scripts/lib/mobile-keyboard-presentation.mjs",
    "scripts/mobile-keyboard-presentation.test.mjs",
    "scripts/lib/android-ime-test-session.sh",
    "scripts/android-ime-test-session.test.sh",
    "scripts/lib/android-package-session.sh",
    "scripts/android-package-session.test.sh",
    "scripts/lib/android-emulator-targeted-preflight.sh",
    "scripts/lib/android-remote-auth-flow.mjs",
    "scripts/lib/android-document-create-pointer-fixture.mjs",
    "scripts/android-document-create-observation.test.mjs",
    "scripts/lib/android-document-create-observation.mjs",
    "scripts/lib/android-document-create-touch.mjs",
    "scripts/lib/android-business-flow-removal-fixture.mjs",
  ];
  assert.ok(local && remote);
  assert.equal(local.environment.DEVE_MOBILE_ANDROID_PRESERVE_PACKAGE, "0");
  assert.equal(remote.environment.DEVE_MOBILE_ANDROID_PRESERVE_PACKAGE, "0");
  for (const artifact of common) {
    assert.ok(local.artifacts.includes(artifact), `local receipt must bind ${artifact}`);
    assert.ok(remote.artifacts.includes(artifact), `remote receipt must bind ${artifact}`);
  }
});

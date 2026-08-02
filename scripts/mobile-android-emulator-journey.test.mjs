import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const read = (path) => fs.readFileSync(new URL(path, import.meta.url), "utf8");
const orchestrator = read("./check-mobile-android-emulator-install-startup-smoke.sh");
const installSmoke = read("./check-mobile-android-install-startup-smoke.sh");
const installRetryTest = read("./android-install-retry.test.sh");
const installRetryLib = read("./lib/android-install-retry.sh");
const guestReadinessTest = read("./android-guest-service-readiness.test.sh");
const guestReadinessLib = read("./lib/android-guest-service-readiness.sh");
const localSmoke = read("./smoke-mobile-android-lifecycle.sh");
const localJourney = read("./smoke-mobile-android-lifecycle.mjs");
const remoteSmoke = read("./smoke-mobile-android-remote-browser.sh");
const remoteJourney = read("./smoke-mobile-android-remote-browser.mjs");
const businessFlow = read("./lib/android-business-flow.mjs");
const writableEvidence = read("./lib/android-writable-evidence.mjs");
const cleanup = read("./cleanup-mobile-android-emulator.sh");
const cleanupTest = read("./android-emulator-cleanup.test.sh");
const ownerLibrary = read("./lib/android-emulator-owner.sh");
const packageBuilder = read("./check-mobile-android-shell-package-build.sh");
const emulatorPin = read("./lib/android-emulator-pin.sh");
const emulatorPinTest = read("./android-emulator-pin.test.sh");
const emulatorRenderer = read("./lib/android-emulator-renderer.sh");
const emulatorRendererTest = read("./android-emulator-renderer.test.sh");
const emulatorFeaturePolicy = read("./lib/android-emulator-feature-policy.sh");
const releaseNativeWorkflow = read("../.github/workflows/release-native.yml");
const nativeTargetHostWorkflow = read("../.github/workflows/native-target-host.yml");
const emulatorHostPreparation = read("./prepare-android-emulator-host.sh");
const producerRegistry = JSON.parse(
  read("../docs/registry/acceptance-producers.json"),
);

test("emulator orchestrator owns both local and remote target lifecycles", () => {
  assert.match(orchestrator, /DEVE_MOBILE_ANDROID_EMULATOR_JOURNEY:-local/);
  assert.match(orchestrator, /case "\$JOURNEY" in[\s\S]*local \| remote/);
  assert.match(orchestrator, /if \[\[ "\$JOURNEY" == "local" \]\]/);
  assert.match(orchestrator, /smoke-mobile-android-lifecycle\.sh/);
  assert.match(orchestrator, /smoke-mobile-android-remote-browser\.sh/);
  assert.match(orchestrator, /run bash "\$ROOT_DIR\/scripts\/android-emulator-cleanup\.test\.sh"/);
  assert.match(orchestrator, /run bash "\$ROOT_DIR\/scripts\/smoke-mobile-android-lifecycle\.sh"/);
  assert.match(orchestrator, /run bash "\$ROOT_DIR\/scripts\/smoke-mobile-android-remote-browser\.sh"/);
  assert.match(orchestrator, /trap cleanup_on_exit EXIT/);
  assert.match(orchestrator, /write_emulator_owner "\$EMULATOR_PID"/);
  assert.match(orchestrator, /DEVE_MOBILE_ANDROID_EMULATOR_PARTITION_MB:-4096/);
  assert.match(orchestrator, /disk\.dataPartition\.size=%sM/);
  assert.doesNotMatch(orchestrator, /^\s*-partition-size /m);
  assert.match(orchestrator, /EMULATOR_PARTITION_MB >= 2048 && EMULATOR_PARTITION_MB <= 8192/);
  assert.match(orchestrator, /shell "df -k \/data"/);
  assert.doesNotMatch(orchestrator, /shell df -k \/data/);
  assert.match(orchestrator, /source "\$ROOT_DIR\/scripts\/lib\/android-emulator-capacity\.sh"/);
  assert.match(orchestrator, /parse_android_emulator_data_capacity/);
  assert.match(orchestrator, /android-emulator-capacity\.test\.sh/);
  assert.match(orchestrator, /available_kib >= 1048576/);
  assert.match(orchestrator, /wait_for_boot\s+verify_emulator_data_capacity/);
  assert.match(orchestrator, /status != 0 && DIAGNOSTICS_PRINTED == 0/);
});

test("local and remote producers use separate claims outputs", () => {
  assert.match(localSmoke, /DEVE_MOBILE_ANDROID_LOCAL_EVIDENCE_PATH/);
  assert.match(remoteSmoke, /DEVE_MOBILE_ANDROID_REMOTE_EVIDENCE_PATH/);
});

test("both Android writable journeys remove the last repo through backend preview and reach NoScope", () => {
  assert.match(businessFlow, /data-deve-repo-switcher-remove/);
  assert.match(businessFlow, /data-deve-repo-removal-confirm/);
  assert.match(businessFlow, /Android last repo NoScope finalization/);
  assert.match(businessFlow, /noScope\.scopeNonce > before\.scopeNonce/);
  assert.match(localJourney, /exerciseAndroidLastRepoRemoval/);
  assert.match(remoteJourney, /exerciseAndroidLastRepoRemoval/);
  assert.match(localJourney, /repoRemovalNoScope/);
  assert.match(remoteJourney, /repoRemovalNoScope/);
  assert.match(localJourney, /repoLifecycle,\s*journey:/);
  assert.match(remoteJourney, /journey,\s*repoLifecycle,/);
  assert.match(writableEvidence, /if \(repoLifecycle\) evidence\.repoLifecycle = repoLifecycle/);
});

test("Android LocalBackend creates the first repo from zero-repo BootstrapUnbound", () => {
  assert.match(businessFlow, /createFirstAndroidRepoFromBootstrapUnbound/);
  assert.match(businessFlow, /initial zero-repo BootstrapUnbound/);
  assert.match(businessFlow, /scopeNonce === 0/);
  assert.match(businessFlow, /zero-repo startup must not claim repo writer readiness/);
  assert.match(businessFlow, /first Create must advance the backend scope nonce/);
  assert.match(localJourney, /createFirstAndroidRepoFromBootstrapUnbound/);
  assert.match(localJourney, /native LocalBackend bootstrap diagnostics/);
});

test("every RemoteBrowser page generation rechecks network, CSP, and native bridge isolation", () => {
  assert.equal(
    [...remoteJourney.matchAll(/await observeRemoteGeneration\(page, observations\);/g)].length,
    2,
  );
  assert.match(remoteJourney, /Network\.requestWillBeSent/);
  assert.match(remoteJourney, /Log\.entryAdded/);
  assert.equal(
    [...remoteJourney.matchAll(/await assertRemoteBridgeIsolation\(page\);/g)].length,
    3,
  );
  assert.match(remoteJourney, /assert\.deepEqual\(bridge, \{ capability: false, facade: false, directFacade: false \}\)/);
});

test("Android producers own a bounded runner finally cleanup", () => {
  const android = producerRegistry.producers.filter(({ producer_id: id }) =>
    id === "android.local-backend" || id === "android.remote-browser");
  assert.equal(android.length, 2);
  for (const producer of android) {
    assert.deepEqual(producer.finally_steps, [{
      program: "bash",
      args: [{ literal: "scripts/cleanup-mobile-android-emulator.sh" }],
    }]);
    assert.ok(producer.artifacts.includes("scripts/cleanup-mobile-android-emulator.sh"));
    assert.ok(producer.artifacts.includes("scripts/android-emulator-capacity.test.sh"));
    assert.ok(producer.artifacts.includes("scripts/android-emulator-cleanup.test.sh"));
    assert.ok(producer.artifacts.includes("scripts/lib/android-emulator-capacity.sh"));
    assert.ok(producer.artifacts.includes("scripts/lib/android-emulator-owner.sh"));
    assert.ok(producer.artifacts.includes("scripts/lib/android-tools.sh"));
  }
  assert.match(cleanup, /source "\$ROOT_DIR\/scripts\/lib\/android-emulator-owner\.sh"/);
  assert.match(ownerLibrary, /DEVE_ACCEPTANCE_PRODUCER_STATE_DIR/);
  assert.match(cleanup, /serial .* belongs to/);
  assert.match(cleanup, /did not stop within/);
  assert.doesNotMatch(cleanup, /kill(?: -9)? "\$emulator_pid"/);
  assert.match(orchestrator, /jobs -pr \| grep -Fx -- "\$EMULATOR_PID"/);
  assert.match(cleanup, /kill_requested=1/);
  assert.match(cleanup, /reserved launch has no termination authority/);
  assert.match(cleanup, /adb devices probe failed/);
  assert.match(cleanupTest, /ANDROID_HOME="\$fake_sdk"/);
  assert.match(cleanupTest, /failed ADB probe was treated as confirmed serial absence/);
  assert.match(cleanupTest, /reserved owner requested emulator termination/);
  assert.match(cleanupTest, /verified shutdown transition did not converge/);
  assert.match(ownerLibrary, /may not escape its state directory/);
});

test("RemoteBrowser receipt binds only its public HTTPS target", () => {
  const remote = producerRegistry.producers.find(({ producer_id: id }) =>
    id === "android.remote-browser");
  assert.deepEqual(remote.bound_env, ["DEVE_MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN"]);
  assert.ok(!remote.bound_env.includes("DEVE_MOBILE_ANDROID_REMOTE_USERNAME"));
  assert.ok(!remote.bound_env.includes("DEVE_MOBILE_ANDROID_REMOTE_PASSWORD"));
  const fixtureArtifacts = remote.artifacts.filter((artifact) =>
    artifact === "scripts/remote-browser-fixture.sh"
      || artifact === "scripts/lib/remote-browser-fixture.sh"
      || artifact.startsWith("scripts/lib/remote-browser-fixture-"));
  assert.deepEqual(fixtureArtifacts, [
    "scripts/remote-browser-fixture.sh",
    "scripts/lib/remote-browser-fixture.sh",
    "scripts/lib/remote-browser-fixture-http.sh",
    "scripts/lib/remote-browser-fixture-json.sh",
    "scripts/lib/remote-browser-fixture-start-supervisor.sh",
  ]);
});

test("runner-owned Android owner paths reject ambient override escape", () => {
  assert.match(ownerLibrary, /DEVE_ACCEPTANCE_PRODUCER_STATE_DIR/);
  assert.match(ownerLibrary, /"\$override" == "\$expected"/);
  assert.match(cleanup, /launch state and PID disagree/);
});

test("Android package build creates the current Web dist before native preflight", () => {
  const requiredBranch = packageBuilder.indexOf(
    'DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1',
  );
  const webDistBuild = packageBuilder.indexOf(
    'run "$ROOT_DIR/scripts/build-web-dist-ci.sh"',
  );
  assert.ok(webDistBuild >= 0, "required package path must build the Web dist");
  assert.ok(requiredBranch >= 0, "required package path must run native preflight");
  assert.ok(
    webDistBuild < requiredBranch,
    "Tauri native-packaging compile must see the current-HEAD Web dist",
  );
});

test("Android emulator runtime tools are required after SDK package repair", () => {
  const installPackages = orchestrator.lastIndexOf("install_sdk_packages");
  const requireEmulator = orchestrator.lastIndexOf(
    'PINNED_EMULATOR_BIN="$(android_resolve_pinned_emulator)"',
  );
  const requireAdb = orchestrator.lastIndexOf("require_android_tool adb");
  assert.ok(installPackages >= 0, "emulator orchestration must install missing SDK packages");
  assert.ok(
    installPackages < requireEmulator,
    "the pinned emulator must be resolved after the SDK package installer can repair it",
  );
  assert.ok(
    installPackages < requireAdb,
    "adb must be checked after the SDK package installer can repair platform-tools",
  );
});

test("local lifecycle smoke fails closed with bounded WebView-socket diagnostics", () => {
  assert.match(
    localSmoke,
    /source "\$ROOT_DIR\/scripts\/lib\/android-startup-diagnostics\.sh"/u,
    "lifecycle smoke must load the bounded startup diagnostics library",
  );
  assert.match(localSmoke, /android_startup_diagnostics_prepare "\$APP_ID"/u);
  assert.match(
    localSmoke,
    /exited while waiting for its debug WebView socket/u,
    "socket wait must fail fast when the app process disappears",
  );
  assert.match(
    localSmoke,
    /restarted while waiting for its debug WebView socket/u,
    "socket wait must fail fast when the app process is replaced",
  );
  assert.match(localSmoke, /android_startup_diagnostics_collect "\$APP_ID"/u);
  assert.match(
    localSmoke,
    /webview_devtools sockets visible to adb/u,
    "missing-socket failure must inventory visible WebView debug sockets",
  );
  assert.match(
    localSmoke,
    /report_missing_webview_socket "\$PID"/u,
    "the socket deadline must route through the bounded diagnostics report",
  );
  assert.match(localSmoke, /report_lifecycle_harness_failure/u);
  assert.match(localSmoke, /android_startup_diagnostics_collect "\$APP_ID"/u);
  assert.match(localSmoke, /if \(\( NODE_STATUS != 0 \)\); then[\s\S]*report_lifecycle_harness_failure/u);
  for (const [name, script] of [["local", localSmoke], ["remote", remoteSmoke]]) {
    assert.match(
      script,
      /shell "cat \/proc\/net\/unix"/u,
      `${name} socket read must quote the remote command against MSYS path conversion`,
    );
    assert.doesNotMatch(
      script,
      /shell cat \/proc\/net\/unix/u,
      `${name} socket read must not pass a bare /proc path through Git Bash`,
    );
  }
});

test("Android APK install retries only after stable package/settings admission", () => {
  assert.match(
    installSmoke,
    /timeout --kill-after="\$\{ADB_KILL_AFTER_SECS\}s"/,
  );
  assert.match(installRetryLib, /INSTALL_RETRY_DEADLINE_SECS=180/);
  assert.match(guestReadinessLib, /ANDROID_GUEST_SERVICE_STABLE_WINDOW_SECS=10/);
  assert.match(guestReadinessLib, /ANDROID_GUEST_SERVICE_POLL_INTERVAL_SECS=2/);
  assert.match(guestReadinessLib, /platform_package == 1/);
  assert.match(guestReadinessLib, /stable_since=""/);
  assert.match(guestReadinessLib, /stable-window=reset/);
  assert.match(installRetryLib, /wait_for_android_guest_services_stable "\$deadline"/);
  assert.match(installRetryLib, /android_guest_services_wait_stable/);
  assert.match(installRetryLib, /retryable_android_settings_provider_install_failure/);
  assert.match(installRetryLib, /for attempt in 1 2 3/);
  assert.match(
    installRetryLib,
    /retryable_android_package_services_ready_install_failure/,
  );
  assert.match(installRetryLib, /recovering_package_services == 1/);
  assert.match(
    installRetryLib,
    /PackageManagerInternal\.freeStorage\(java\.lang\.String, long, int\)/,
  );
  assert.match(installRetryLib, /StorageManagerService\\\.allocateBytes/);
  assert.match(installRetryLib, /PackageInstallerSession\\\.doWriteInternal/);
  assert.match(installRetryLib, /streamed_install <= 1/);
  assert.ok(
    installRetryLib.includes("Broken pipe \\(32\\)$/ {"),
    "retry classifier must anchor the exact Broken pipe (32) line",
  );
  assert.ok(
    installRetryLib.includes("Can'\\''t find service: package$/ {"),
    "retry classifier must anchor the exact missing package-service line",
  );
  assert.match(installRetryLib, /broken_pipe \+ package_service_missing == 1/);
  assert.match(
    installRetryLib,
    /wait_for_android_launcher_activity "\$deadline"/,
  );
  assert.match(
    installRetryLib,
    /cmd package resolve-activity[\s\S]*?--components[\s\S]*?android\.intent\.action\.MAIN[\s\S]*?android\.intent\.category\.LAUNCHER/,
  );
  assert.match(installRetryLib, /\[\[ "\$output" == "No activity found" \]\] \|\| return 1/);
  assert.match(installRetryLib, /adb_retry_timed "\$deadline" wait-for-device/);
  assert.match(
    guestReadinessLib,
    /shell cmd package list packages/,
  );
  assert.match(guestReadinessLib, /shell settings get global device_provisioned/);
  assert.match(orchestrator, /android_emulator_wait_for_guest_services_stable/);
  assert.match(orchestrator, /android-guest-service-readiness\.test\.sh/);
  assert.match(
    installRetryLib,
    /non-transport Android install failures must remain fail-closed/,
  );
  for (const [hostName, host, expectedPrefix] of [
    ["install-startup smoke", installSmoke, "mobile-android-install-startup-smoke-check"],
    ["local lifecycle smoke", localSmoke, "mobile-android-lifecycle-smoke"],
    ["remote-browser smoke", remoteSmoke, "mobile-android-remote-browser-smoke"],
  ]) {
    assert.match(
      host,
      /source "\$ROOT_DIR\/scripts\/lib\/android-install-retry\.sh"/,
      `${hostName} must source the shared install retry library`,
    );
    assert.ok(
      host.includes(`ANDROID_INSTALL_RETRY_LOG_PREFIX="${expectedPrefix}"`),
      `${hostName} must attribute install retry progress to its own log prefix`,
    );
    assert.match(
      host,
      /^(run )?install_apk$/m,
      `${hostName} must install through the bounded retry entry point`,
    );
    assert.ok(
      !/adb_cmd install -r/.test(host),
      `${hostName} must not install the APK outside the bounded retry path`,
    );
  }
  for (const producerId of ["android.local-backend", "android.remote-browser"]) {
    const producer = producerRegistry.producers.find(
      ({ producer_id: id }) => id === producerId,
    );
    assert.ok(
      producer.artifacts.includes(
        "scripts/check-mobile-android-install-startup-smoke.sh",
      ),
      `${producerId} receipt must bind the APK install retry implementation`,
    );
    assert.ok(
      producer.artifacts.includes("scripts/android-install-retry.test.sh"),
      `${producerId} receipt must bind the APK install retry state matrix`,
    );
    assert.ok(
      producer.artifacts.includes("scripts/lib/android-install-retry.sh"),
      `${producerId} receipt must bind the shared install retry library`,
    );
    assert.ok(
      producer.artifacts.includes("scripts/android-guest-service-readiness.test.sh"),
      `${producerId} receipt must bind the stable guest-service state matrix`,
    );
    assert.ok(
      producer.artifacts.includes("scripts/lib/android-guest-service-readiness.sh"),
      `${producerId} receipt must bind the stable guest-service boundary`,
    );
  }
  assert.match(installRetryTest, /run_case success-after-retry 0 2/);
  assert.match(installRetryTest, /run_case always-broken 1 3/);
  assert.match(installRetryTest, /run_case timeout 124 1/);
  assert.match(installRetryTest, /run_case package-internal-recover 0 3/);
  assert.match(installRetryTest, /run_case package-internal-first 1 1/);
  assert.match(installRetryTest, /run_case launcher-delayed 0 1/);
  assert.match(installRetryTest, /run_case launcher-deadline 124 1/);
  assert.match(installRetryTest, /run_case launcher-sleep-fail 42 1/);
  assert.match(guestReadinessTest, /expect_stable package-reset 14 8 7 1/);
  assert.match(guestReadinessTest, /expect_stable settings-reset 14 8 8 1/);
  assert.match(guestReadinessTest, /package-timeout 30 124/);
  assert.match(guestReadinessTest, /package-ready-then-timeout 30 124/);
  assert.match(guestReadinessTest, /settings-ready-then-timeout 30 124/);
  assert.match(guestReadinessTest, /guard-fail 30 23/);
  assert.match(guestReadinessTest, /guard-final-fail 30 23/);
  assert.match(guestReadinessTest, /package-success-mixed 30 1/);
  assert.match(installRetryLib, /remaining > kill_after_secs/);
});

test("emulator gate pins the exact stable emulator build fail-closed", () => {
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_VERSION="36\.6\.11\.0"/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_BUILD_ID="15507667"/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_SHA256_LINUX="[0-9a-f]{64}"/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_SHA256_WINDOWS="[0-9a-f]{64}"/);
  // checksum precedes extraction, then one build-scoped publisher owns rename
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
  // every resolution path re-asserts the version banner; no silent fallback
  assert.match(emulatorPin, /android_emulator_pin_matches "\$binary"/);
  assert.match(emulatorPin, /does not match pin \$ANDROID_EMULATOR_PIN_VERSION/);
  assert.match(emulatorPin, /downloaded emulator does not match pin/);
  // identity comes from a bounded canonical banner, not loose tokens or exit
  // status, and every mismatch reports a bounded diagnostic.
  assert.match(emulatorPin, /timeout --signal=TERM --kill-after=5s/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_PROBE_MAX_BYTES/);
  assert.match(emulatorPin, /Android emulator version \[0-9\]/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_LAST_PROBE/);
  assert.match(emulatorPinTest, /expect_match nonzero-canonical/);
  assert.match(emulatorPinTest, /expect_reject loose-tokens/);
  assert.match(emulatorPinTest, /expect_reject timeout/);
  assert.match(emulatorPinTest, /expect_reject oversized/);
  // the shared SDK is never mutated: installs go to the private cache root
  assert.match(emulatorPin, /DEVE_MOBILE_ANDROID_EMULATOR_PIN_DIR:-\$HOME\/\.cache\/deve-android-emulator-pin/);
  // the lib only queries the SDK path read-only; it never runs SDK tools
  assert.doesNotMatch(emulatorPin, /android_run_tool|sdkmanager_cmd/);
});

test("Android target-host workflows share the pinned emulator host preparation", () => {
  const mobileAndroidJob = releaseNativeWorkflow.slice(
    releaseNativeWorkflow.indexOf("  mobile-android:"),
  );
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
  assert.doesNotMatch(
    orchestrator,
    /\$PINNED_EMULATOR_BIN" -version/,
    "the orchestrator must not repeat the bounded identity probe without its limits",
  );
  // diagnostics may list AVDs via the SDK binary; the launch path must
  // only ever exec the resolved pin
  assert.match(orchestrator, /"\$PINNED_EMULATOR_BIN" "\$@"/);
});

test("emulator gate pins the API 37.0 image with swangle and 4096 MiB", () => {
  assert.match(orchestrator, /API_LEVEL="\$\{DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL:-37\.0\}"/);
  assert.match(orchestrator, /EMULATOR_RAM_MB="\$\{DEVE_MOBILE_ANDROID_EMULATOR_RAM_MB:-4096\}"/);
  assert.match(orchestrator, /-gpu swangle/);
  // the legacy translator path aborts guest surfaceflinger on this image
  assert.doesNotMatch(orchestrator, /swiftshader_indirect/);
  // actual renderer/ICD selection must be proven from a bounded log prefix
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
    const producer = producerRegistry.producers.find(
      (candidate) => candidate.producer_id === producerId,
    );
    assert.ok(producer, `${producerId} must exist in the producer registry`);
    assert.ok(
      producer.artifacts.includes("scripts/lib/android-emulator-pin.sh"),
      `${producerId} receipt must bind the emulator pin library`,
    );
    for (const artifact of [
      "scripts/android-emulator-pin.test.sh",
      "scripts/android-emulator-renderer.test.sh",
      "scripts/android-emulator-feature-policy.test.sh",
      "scripts/lib/android-emulator-renderer.sh",
      "scripts/lib/android-emulator-feature-policy.sh",
      "scripts/lib/android-emulator-diagnostics.sh",
    ]) {
      assert.ok(
        producer.artifacts.includes(artifact),
        `${producerId} receipt must bind ${artifact}`,
      );
    }
  }
});

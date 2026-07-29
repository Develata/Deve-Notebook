import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const orchestrator = fs.readFileSync(
  new URL("./check-mobile-android-emulator-install-startup-smoke.sh", import.meta.url),
  "utf8",
);
const installSmoke = fs.readFileSync(
  new URL("./check-mobile-android-install-startup-smoke.sh", import.meta.url),
  "utf8",
);
const installRetryTest = fs.readFileSync(
  new URL("./android-install-retry.test.sh", import.meta.url),
  "utf8",
);
const installRetryLib = fs.readFileSync(
  new URL("./lib/android-install-retry.sh", import.meta.url),
  "utf8",
);
const localSmoke = fs.readFileSync(
  new URL("./smoke-mobile-android-lifecycle.sh", import.meta.url),
  "utf8",
);
const localJourney = fs.readFileSync(
  new URL("./smoke-mobile-android-lifecycle.mjs", import.meta.url),
  "utf8",
);
const remoteSmoke = fs.readFileSync(
  new URL("./smoke-mobile-android-remote-browser.sh", import.meta.url),
  "utf8",
);
const remoteJourney = fs.readFileSync(
  new URL("./smoke-mobile-android-remote-browser.mjs", import.meta.url),
  "utf8",
);
const businessFlow = fs.readFileSync(
  new URL("./lib/android-business-flow.mjs", import.meta.url),
  "utf8",
);
const writableEvidence = fs.readFileSync(
  new URL("./lib/android-writable-evidence.mjs", import.meta.url),
  "utf8",
);
const cleanup = fs.readFileSync(
  new URL("./cleanup-mobile-android-emulator.sh", import.meta.url),
  "utf8",
);
const cleanupTest = fs.readFileSync(
  new URL("./android-emulator-cleanup.test.sh", import.meta.url),
  "utf8",
);
const ownerLibrary = fs.readFileSync(
  new URL("./lib/android-emulator-owner.sh", import.meta.url),
  "utf8",
);
const packageBuilder = fs.readFileSync(
  new URL("./check-mobile-android-shell-package-build.sh", import.meta.url),
  "utf8",
);
const emulatorPin = fs.readFileSync(
  new URL("./lib/android-emulator-pin.sh", import.meta.url),
  "utf8",
);
const producerRegistry = JSON.parse(fs.readFileSync(
  new URL("../docs/registry/acceptance-producers.json", import.meta.url),
  "utf8",
));

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

test("Android APK install retries only exact package-service recovery failures", () => {
  assert.match(
    installSmoke,
    /timeout --kill-after="\$\{ADB_KILL_AFTER_SECS\}s"/,
  );
  assert.match(installRetryLib, /INSTALL_RETRY_DEADLINE_SECS=180/);
  assert.match(installRetryLib, /PACKAGE_SERVICE_READY_ATTEMPTS=10/);
  assert.match(installRetryLib, /PACKAGE_SERVICE_READY_INTERVAL_SECS=2/);
  assert.match(installRetryLib, /wait_for_android_package_service "\$deadline"/);
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
  assert.match(
    installRetryLib,
    /broken_pipe \+ package_service_missing == 1/,
  );
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
    installRetryLib,
    /adb_retry_timed "\$deadline" shell cmd package list packages/,
  );
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
  }
  assert.match(installRetryTest, /run_case success-after-retry 0 2 5/);
  assert.match(installRetryTest, /run_case always-broken 1 3 7/);
  assert.match(installRetryTest, /run_case timeout 124 1 1/);
  assert.match(installRetryTest, /run_case pipeline 1 1 1/);
  assert.match(installRetryTest, /run_case wait-fail 17 1 2/);
  assert.match(installRetryTest, /run_case package-internal-recover 0 3 8 2/);
  assert.match(installRetryTest, /run_case package-internal-first 1 1 1 0/);
  assert.match(installRetryTest, /run_case package-internal-mixed 1 2 4 1/);
  assert.match(installRetryTest, /run_case launcher-delayed 0 1 4 2/);
  assert.match(installRetryTest, /run_case launcher-mixed 1 1 2 0/);
  assert.match(installRetryTest, /run_case launcher-other 1 1 2 0/);
  assert.match(installRetryTest, /run_case launcher-timeout 124 1 2 0/);
  assert.match(installRetryTest, /run_case launcher-unavailable 1 1 11 9/);
  assert.match(installRetryTest, /run_case launcher-deadline 124 1 2 0/);
  assert.match(installRetryTest, /run_case launcher-sleep-fail 42 1 2 1/);
});

test("emulator gate pins the exact stable emulator build fail-closed", () => {
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_VERSION="36\.6\.11\.0"/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_BUILD_ID="15507667"/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_SHA256_LINUX="[0-9a-f]{64}"/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_SHA256_WINDOWS="[0-9a-f]{64}"/);
  // checksum precedes extraction, and extraction lands via atomic rename
  assert.match(emulatorPin, /sha256sum -c --quiet -/);
  assert.match(
    emulatorPin,
    /sha256sum[\s\S]*unzip[\s\S]*mv -f -- "\$extracted" "\$cache_root\/\$ANDROID_EMULATOR_PIN_BUILD_ID"/,
  );
  // every resolution path re-asserts the version banner; no silent fallback
  assert.match(emulatorPin, /android_emulator_pin_matches "\$binary"/);
  assert.match(emulatorPin, /does not match pin \$ANDROID_EMULATOR_PIN_VERSION/);
  assert.match(emulatorPin, /downloaded emulator does not match pin/);
  // identity comes from banner tokens, never from the probe's exit status,
  // and every mismatch reports what the candidate binary actually printed
  assert.match(emulatorPin, /rc=\$\?/);
  assert.match(emulatorPin, /ANDROID_EMULATOR_PIN_LAST_PROBE/);
  assert.doesNotMatch(emulatorPin, /head -n 1/);
  // the shared SDK is never mutated: installs go to the private cache root
  assert.match(emulatorPin, /DEVE_MOBILE_ANDROID_EMULATOR_PIN_DIR:-\$HOME\/\.cache\/deve-android-emulator-pin/);
  // the lib only queries the SDK path read-only; it never runs SDK tools
  assert.doesNotMatch(emulatorPin, /android_run_tool|sdkmanager_cmd/);
});

test("emulator orchestrator launches only the resolved pinned binary", () => {
  assert.match(orchestrator, /source "\$ROOT_DIR\/scripts\/lib\/android-emulator-pin\.sh"/);
  assert.match(orchestrator, /PINNED_EMULATOR_BIN="\$\(android_resolve_pinned_emulator\)"/);
  assert.match(orchestrator, /pinned Android emulator was not resolved before emulator_cmd/);
  assert.match(orchestrator, /pinned emulator: \$PINNED_EMULATOR_BIN/);
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
  // actual renderer/ICD selection is recorded as gate evidence
  assert.match(orchestrator, /vulkan_mode_selected\|gles_mode_selected\|setCurrentRenderer/);
});

test("android producers bind the emulator pin library", () => {
  for (const producerId of ["android.local-backend", "android.remote-browser"]) {
    const producer = producerRegistry.producers.find(
      (candidate) => candidate.producer_id === producerId,
    );
    assert.ok(producer, `${producerId} must exist in the producer registry`);
    assert.ok(
      producer.artifacts.includes("scripts/lib/android-emulator-pin.sh"),
      `${producerId} receipt must bind the emulator pin library`,
    );
  }
});

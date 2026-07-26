import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const orchestrator = fs.readFileSync(
  new URL("./check-mobile-android-emulator-install-startup-smoke.sh", import.meta.url),
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
  assert.match(orchestrator, /-partition-size "\$EMULATOR_PARTITION_MB"/);
  assert.match(orchestrator, /EMULATOR_PARTITION_MB >= 2048 && EMULATOR_PARTITION_MB <= 8192/);
  assert.match(orchestrator, /shell df -k \/data/);
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
    assert.ok(producer.artifacts.includes("scripts/android-emulator-cleanup.test.sh"));
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
  const requireEmulator = orchestrator.lastIndexOf("require_android_tool emulator");
  const requireAdb = orchestrator.lastIndexOf("require_android_tool adb");
  assert.ok(installPackages >= 0, "emulator orchestration must install missing SDK packages");
  assert.ok(
    installPackages < requireEmulator,
    "the emulator binary must be checked after the SDK package installer can repair it",
  );
  assert.ok(
    installPackages < requireAdb,
    "adb must be checked after the SDK package installer can repair platform-tools",
  );
});

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { probeWebCryptoEd25519 } from "./lib/webcrypto-capability.mjs";
import { evaluateWritableProbeExpectation } from "./lib/android-target-capability.mjs";
import { writeAndroidWritableEvidence } from "./lib/android-writable-evidence.mjs";
import {
  findStableAppPage,
  isExpectedCdpTargetRetirement,
  reloadPageAndWaitForNewMainDocument,
  remoteEntrySurfacePresent,
  visibleElement,
} from "./lib/android-webview-cdp.mjs";
import {
  commitAndroidChange,
  createFirstAndroidRepoFromBootstrapUnbound,
  createAndroidDocument,
  dispatchWebViewText,
  exerciseAndroidLastRepoRemoval,
  loginAndroidRemote,
  waitForWritableEditor as waitForWritableAndroidEditor,
} from "./lib/android-business-flow.mjs";
import {
  observeAnchoredAndroidAppProcess,
  probeAndroidAppProcess,
  waitForAnchoredAndroidAppProcessExit,
} from "./lib/android-app-process-observation.mjs";
import {
  androidLogcatContains,
  androidLogcatMatchStates,
} from "./lib/android-logcat-observation.mjs";
import {
  ANDROID_EMBEDDED_BACKEND_STARTED_MARKER,
  ANDROID_REMOTE_BROWSER_MODE_MARKER,
  requireAndroidRemoteBrowserModeEvidence,
} from "./lib/android-native-mode-evidence.mjs";
import { typeAndroidEditorText } from "./lib/mobile-webview-interaction.mjs";
import { waitForAcceptedAndroidPresentation } from "./lib/android-presentation-proof.mjs";

const timeoutMs = Number(process.env.DEVE_MOBILE_ANDROID_REMOTE_TIMEOUT_MS ?? "120000");
const cdpEndpoint = process.env.DEVE_MOBILE_ANDROID_CDP_ENDPOINT;
const remoteOrigin = process.env.DEVE_MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN;
const username = process.env.DEVE_MOBILE_ANDROID_REMOTE_USERNAME;
const password = process.env.DEVE_MOBILE_ANDROID_REMOTE_PASSWORD;
const targetFactsPath = process.env.DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH;
const evidencePath = process.env.DEVE_MOBILE_ANDROID_EVIDENCE_PATH;
const adb = process.env.DEVE_MOBILE_ANDROID_ADB_BIN;
const serial = process.env.DEVE_MOBILE_ANDROID_SERIAL;
const appId = process.env.DEVE_MOBILE_ANDROID_APP_ID ?? "dev.deve.notebook.mobile";
const expectedAppPid = process.env.DEVE_MOBILE_ANDROID_EXPECTED_APP_PID;
const deadline = Date.now() + timeoutMs;

function remainingMs() {
  const remaining = deadline - Date.now();
  if (remaining <= 0) throw new Error("Android RemoteBrowser harness deadline exhausted");
  return remaining;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function adbCommand(...args) {
  execFileSync(adb, ["-s", serial, ...args], { stdio: "inherit", timeout: remainingMs() });
}

function adbOutput(...args) {
  return execFileSync(adb, ["-s", serial, ...args], {
    encoding: "utf8",
    timeout: remainingMs(),
  }).replaceAll("\r", "");
}

function appPid(probeTimeoutMs = remainingMs()) {
  return probeAndroidAppProcess({
    adb,
    serial,
    appId,
    timeoutMs: Math.min(remainingMs(), probeTimeoutMs),
  });
}

function logcatContains(pattern) {
  return androidLogcatContains({
    adb,
    serial,
    pattern,
    timeoutMs: Math.min(remainingMs(), 10_000),
  });
}

function logcatModeMatchStates() {
  return androidLogcatMatchStates({
    adb,
    serial,
    patterns: [
      ANDROID_REMOTE_BROWSER_MODE_MARKER,
      ANDROID_EMBEDDED_BACKEND_STARTED_MARKER,
    ],
    timeoutMs: Math.min(remainingMs(), 10_000),
  });
}

async function withDeadline(label, promise, limit = remainingMs()) {
  let timer;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`timeout during ${label}`)), Math.min(limit, remainingMs()));
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

async function waitUntil(label, predicate, timeout = Math.min(timeoutMs, 30000)) {
  const stop = Math.min(Date.now() + timeout, deadline);
  let lastError;
  while (Date.now() < stop) {
    try {
      const value = await withDeadline(label, Promise.resolve().then(predicate), stop - Date.now());
      if (value) return value;
    } catch (error) {
      lastError = error;
    }
    await delay(250);
  }
  throw new Error(`timeout waiting for ${label}${lastError ? `: ${lastError.message}` : ""}`);
}

async function attachRemotePage() {
  const page = await findStableAppPage({
    cdpEndpoint,
    expectedOrigin: remoteOrigin,
    requiredSurface: "remote-entry",
    withDeadline,
  });
  await page.evaluate(`globalThis.__deveVisibleElement = ${visibleElement.toString()}`);
  return page;
}

async function attachLocalPage() {
  const page = await findStableAppPage({
    cdpEndpoint,
    expectedOrigin: undefined,
    requiredSurface: "sync",
    withDeadline,
  });
  await page.evaluate(`globalThis.__deveVisibleElement = ${visibleElement.toString()}`);
  return page;
}

async function observeRemoteGeneration(page, observations) {
  page.on("Network.requestWillBeSent", ({ request }) => {
    observations.requests.push(request?.url ?? "");
  });
  page.on("Log.entryAdded", ({ entry }) => {
    if (entry?.level === "error") observations.consoleErrors.push(entry.text ?? "");
  });
  await page.send("Network.enable");
  await page.send("Log.enable");
  await reloadPageAndWaitForNewMainDocument(page, withDeadline, 30000);
  await waitUntil(
    "RemoteBrowser DOM reload",
    () => page.call(remoteEntrySurfacePresent, remoteOrigin),
    30000,
  );
  await page.evaluate(`globalThis.__deveVisibleElement = ${visibleElement.toString()}`);
  await assertRemoteBridgeIsolation(page);
}

async function assertRemoteBridgeIsolation(page) {
  const bridge = await page.call(() => ({
    capability: globalThis.__DEVE_NATIVE_BOOTSTRAP?.capabilities
      ?.backend_preference_control === true,
    facade: Boolean(globalThis.__deveWebBridge?.get?.("__DEVE_NATIVE_BACKEND_CONFIG__")),
    directFacade: Boolean(globalThis.__DEVE_NATIVE_BACKEND_CONFIG__),
  }));
  assert.deepEqual(bridge, { capability: false, facade: false, directFacade: false });
}

async function readScope(page) {
  return page.call(() => {
    const status = document.querySelector("[data-deve-sync-status]");
    return {
      status: status?.getAttribute("data-deve-sync-status") ?? null,
      repoId: status?.getAttribute("data-deve-repo-id") ?? null,
      scopeNonce: Number(status?.getAttribute("data-deve-scope-nonce")),
    };
  });
}

function findNativeRecoveryButtonBounds(xml) {
  const node = xml
    .match(/<node\b[^>]*>/g)
    ?.find((candidate) =>
      /(?:text|content-desc)="Use Local Backend"/.test(candidate));
  const bounds = node?.match(/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/);
  if (!bounds) throw new Error(`native Use Local Backend control unavailable; ui=${xml.slice(0, 1000)}`);
  const [, left, top, right, bottom] = bounds.map(Number);
  return {
    x: Math.floor((left + right) / 2),
    y: Math.floor((top + bottom) / 2),
  };
}

function tapNativeRecoveryControl() {
  const dumpPath = "/sdcard/deve-native-recovery.xml";
  adbCommand("shell", "uiautomator", "dump", dumpPath);
  const bounds = findNativeRecoveryButtonBounds(adbOutput("exec-out", "cat", dumpPath));
  adbCommand("shell", "input", "tap", String(bounds.x), String(bounds.y));
}

async function nativeInvoke(page, command, args = {}) {
  const outcome = await page.call(async (invokeCommand, invokeArgs) => {
    try {
      const invoke = window.__TAURI_INTERNALS__?.invoke;
      if (typeof invoke !== "function") throw new Error("Tauri invoke bridge unavailable");
      return {
        ok: true,
        value: await invoke(`plugin:deve-native-backend-commands|${invokeCommand}`, invokeArgs),
      };
    } catch (error) {
      return { ok: false, error: String(error) };
    }
  }, command, args);
  if (!outcome.ok) throw new Error(`native command ${command} failed: ${outcome.error}`);
  return outcome.value;
}

async function inputAndroidEditorText(content, _point, page, expectedDocId = null) {
  return typeAndroidEditorText(page, content, {
    delay,
    waitForWritableEditor: (editorPage, requiredDocId) =>
      waitForWritableAndroidEditor(editorPage, waitUntil, 30000, requiredDocId),
    inputText: (value) => dispatchWebViewText(page, value),
    expectedDocId,
  });
}

async function main() {
  if (!cdpEndpoint || !remoteOrigin || !username || !password || !adb || !serial
    || !/^[1-9][0-9]*$/.test(expectedAppPid ?? "")) {
    throw new Error(
      "CDP endpoint, remote origin, credentials, adb, serial, and admitted app PID are required",
    );
  }
  assert.equal(new URL(remoteOrigin).protocol, "https:");
  const observations = { requests: [], consoleErrors: [] };
  let page = await attachRemotePage();
  await observeRemoteGeneration(page, observations);
  const capability = await withDeadline(
    "RemoteBrowser Ed25519 probe",
    page.call(probeWebCryptoEd25519),
    30000,
  );
  evaluateWritableProbeExpectation(true, capability);
  await loginAndroidRemote(page, remoteOrigin, username, password, waitUntil);
  const nativePresentation = await waitForAcceptedAndroidPresentation(page, waitUntil);
  console.log(
    `mobile-android-remote-browser: system gesture presentation ${JSON.stringify(nativePresentation)}`,
  );
  const stamp = Date.now();
  await createAndroidDocument(
    page,
    `android-remote-${stamp}.md`,
    `Android RemoteBrowser smoke ${stamp}`,
    {
      waitUntil,
      inputEditorText: inputAndroidEditorText,
    },
  );
  await commitAndroidChange(page, `android remote smoke ${stamp}`, { waitUntil, delay });
  await assertRemoteBridgeIsolation(page);

  const remoteRuntime = await page.call(() => ({
    origin: location.origin,
  }));
  assert.equal(remoteRuntime.origin, new URL(remoteOrigin).origin);

  adbCommand("shell", "input", "keyevent", "3");
  await page.close();
  await delay(500);
  adbCommand("shell", "monkey", "-p", appId, "-c", "android.intent.category.LAUNCHER", "1");
  page = await attachRemotePage();
  await observeRemoteGeneration(page, observations);
  await loginAndroidRemote(page, remoteOrigin, username, password, waitUntil);
  await assertRemoteBridgeIsolation(page);
  assert.equal(new URL(await page.call(() => location.href)).origin, new URL(remoteOrigin).origin);
  const remoteScope = await readScope(page);
  assert.equal(remoteScope.status, "ready");
  assert.ok(remoteScope.repoId, "RemoteBrowser must expose a repo-scoped ready handshake");
  assert.ok(Number.isInteger(remoteScope.scopeNonce) && remoteScope.scopeNonce > 0);
  const repoLifecycle = await exerciseAndroidLastRepoRemoval(page, {
    waitUntil,
    expectedRepoId: remoteScope.repoId,
    minimumScopeNonce: remoteScope.scopeNonce,
  });

  const ipcRequests = observations.requests.filter((url) => url.includes("ipc.localhost"));
  const ipcCspErrors = observations.consoleErrors.filter((message) =>
    /ipc\.localhost|content security policy|refused to connect/i.test(message));
  assert.deepEqual(ipcRequests, []);
  assert.deepEqual(ipcCspErrors, []);

  const remotePid = await observeAnchoredAndroidAppProcess(expectedAppPid, {
    probe: appPid,
    delay,
  });
  requireAndroidRemoteBrowserModeEvidence(await logcatModeMatchStates());
  tapNativeRecoveryControl();
  await page.close().catch(() => {});
  // Android owns Activity retirement; Chromium may keep a destroyed WebView's
  // discovery target cached after Wry has removed the corresponding window.
  // Admit only the fresh bundled-local target, then verify the ordered native
  // transition snapshot instead of treating debugger-cache lifetime as product
  // lifecycle authority.
  page = await attachLocalPage();
  const localRuntime = await waitUntil("fresh LocalBackend bootstrap", async () => {
    const runtime = await page.call(() => ({
      origin: location.origin,
      httpBase: globalThis.__DEVE_NATIVE_BOOTSTRAP?.http_base ?? null,
      sessionBound: globalThis.__DEVE_NATIVE_BOOTSTRAP?.session_bound === true,
      nativeCapability: globalThis.__DEVE_NATIVE_BOOTSTRAP?.capabilities
        ?.backend_preference_control === true,
      nativeFacade: Boolean(globalThis.__deveWebBridge?.get?.("__DEVE_NATIVE_BACKEND_CONFIG__")),
    }));
    return runtime.origin === "http://tauri.localhost"
      && /^http:\/\/127\.0\.0\.1:\d+$/.test(runtime.httpBase ?? "")
      && runtime.sessionBound
      && runtime.nativeCapability
      && runtime.nativeFacade
      ? runtime
      : null;
  }, 60000);
  const service = await waitUntil("fresh LocalBackend service state", async () => {
    const state = await nativeInvoke(page, "native_backend_get_service_state");
    return state?.backend_running
      && state.service_state === "endpoint_session_ready"
      && state.session_generation === 1
      && state.endpoint === localRuntime.httpBase
      ? state
      : null;
  }, 30000);
  const localRepoLifecycle = await createFirstAndroidRepoFromBootstrapUnbound(
    page,
    `android-local-recovery-${stamp}`,
    { waitUntil },
  );
  assert.equal(localRepoLifecycle.defaultRepoAbsent, true);
  assert.equal(localRepoLifecycle.aliasCount, 1);
  const localScope = localRepoLifecycle.created;
  assert.equal(localScope.status, "ready");
  assert.ok(localScope.repoId, "LocalBackend first Create must establish a repo scope");
  assert.ok(Number.isSafeInteger(localScope.scopeNonce) && localScope.scopeNonce > 0);
  const transition = await nativeInvoke(page, "native_backend_get_recovery_state");
  assert.ok(
    Number.isSafeInteger(transition.recoveryId) && transition.recoveryId > 0,
    "native recovery transition must bind a positive coordinator attempt ID",
  );
  assert.equal(transition.phase, "local_window_created");
  assert.equal(transition.remoteSurfaceRetired, true);
  assert.equal(transition.preferenceCommittedAfterRemoteRetirement, true);
  assert.equal(transition.localPluginsRegisteredAfterRemoteRetirement, true);
  assert.equal(transition.supervisorManaged, true);
  assert.equal(transition.localWindowCreated, true);
  assert.equal(transition.activeRuntimeOwners, 1);
  assert.equal(transition.lastError, null);
  assert.equal(
    await logcatContains(/deve_mobile RemoteBrowser recovered to fresh LocalBackend runtime/),
    true,
    "native recovery must emit its bounded LocalBackend completion diagnostic",
  );
  const localPid = await observeAnchoredAndroidAppProcess(expectedAppPid, {
    probe: appPid,
    delay,
  });
  assert.equal(localPid, remotePid, "mobile recovery must not orphan or replace the app process");
  assert.notEqual(localRuntime.origin, remoteRuntime.origin);
  const authorityTupleChanged = remoteRuntime.origin !== localRuntime.origin
    && `${remoteScope.repoId}:${remoteScope.scopeNonce}`
      !== `${localScope.repoId}:${localScope.scopeNonce}`;
  assert.equal(authorityTupleChanged, true, "remote authority tuple must not be reused locally");

  await nativeInvoke(page, "native_backend_debug_request_exit").catch((error) => {
    if (!isExpectedCdpTargetRetirement(error)) throw error;
  });
  await page.close().catch(() => {});
  const processExitedAfterGracefulShutdown = await waitForAnchoredAndroidAppProcessExit(
    expectedAppPid,
    {
      probe: appPid,
      delay,
      timeoutMs: Math.min(remainingMs(), 30000),
    },
  );
  const recovery = {
    transition,
    remote: {
      origin: remoteRuntime.origin,
      repoId: remoteScope.repoId,
      scopeNonce: remoteScope.scopeNonce,
    },
    local: {
      origin: localRuntime.origin,
      endpoint: service.endpoint,
      sessionGeneration: service.session_generation,
      bootstrapUnbound: {
        status: localRepoLifecycle.initial.status,
        repoIdEmpty: localRepoLifecycle.initial.repoIdRaw === "",
        scopeNonce: localRepoLifecycle.initial.scopeNonce,
        defaultRepoAbsent: localRepoLifecycle.defaultRepoAbsent,
      },
      status: localScope.status,
      repoId: localScope.repoId,
      scopeNonce: localScope.scopeNonce,
    },
    authorityTupleChanged,
    appPidStable: localPid === remotePid,
    processExitedAfterGracefulShutdown,
  };
  const journey = {
    loginOrNativeSession: true,
    edit: true,
    commitHistory: true,
    backgroundResume: true,
    nativeSystemGestureInsetsAcceptedAfterReload: true,
    repoRemovalNoScope: repoLifecycle.noScope,
    zeroNativeIpc: ipcRequests.length === 0 && ipcCspErrors.length === 0,
    nativeLocalRecovery: transition.phase === "local_window_created",
    remoteSurfaceDestroyedBeforeLocalIpc: transition.remoteSurfaceRetired
      && transition.localPluginsRegisteredAfterRemoteRetirement,
    freshLocalBootstrapUnboundBeforeFirstCreate: localRepoLifecycle.initial.status
      === "handshaking-repo"
      && localRepoLifecycle.initial.repoIdRaw === ""
      && localRepoLifecycle.initial.scopeNonce === 0
      && localRepoLifecycle.defaultRepoAbsent,
    freshLocalEndpointSessionScope: Boolean(localScope.repoId)
      && localScope.scopeNonce > 0
      && service.session_generation === 1,
    remoteAuthorityNotReused: authorityTupleChanged,
    noOrphanEmbeddedRuntime: transition.activeRuntimeOwners === 1
      && processExitedAfterGracefulShutdown,
    writableLifecycleComplete: true,
  };
  assert.ok(Object.values(journey).every((value) => value === true));
  writeAndroidWritableEvidence({
    evidencePath,
    targetFactsPath,
    producer: "smoke-mobile-android-remote-browser",
    mode: "remote-browser",
    webcrypto: capability,
    journey,
    repoLifecycle,
    recovery,
  });
  console.log("mobile-android-remote-browser: ok");
}

main().then(
  () => process.exit(0),
  (error) => {
    console.error(`mobile-android-remote-browser: ${error.stack ?? error.message}`);
    process.exit(1);
  },
);

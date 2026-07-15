import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { probeWebCryptoEd25519 } from "./lib/webcrypto-capability.mjs";
import { evaluateWritableProbeExpectation } from "./lib/android-target-capability.mjs";
import { findStableAppPage, visibleElement } from "./lib/android-webview-cdp.mjs";
import {
  commitAndroidChange,
  createAndroidDocument,
  dispatchWebViewText,
  loginAndroidRemote,
} from "./lib/android-business-flow.mjs";

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

function appPid() {
  return adbOutput("shell", "sh", "-c", `pidof ${appId} 2>/dev/null || true`)
    .trim()
    .split(/\s+/)
    .filter(Boolean)[0] ?? "";
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
    withDeadline,
    waitUntil,
  });
  await page.evaluate(`globalThis.__deveVisibleElement = ${visibleElement.toString()}`);
  return page;
}

async function attachLocalPage() {
  const page = await findStableAppPage({
    cdpEndpoint,
    expectedOrigin: undefined,
    withDeadline,
    waitUntil,
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
  await page.send("Page.reload", { ignoreCache: true });
  await waitUntil("RemoteBrowser DOM reload", () => page.call(() =>
    Boolean(document.querySelector("[data-deve-sync-status]"))), 30000);
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

async function listCdpTargets() {
  const response = await withDeadline(
    "Android WebView target discovery",
    fetch(`${cdpEndpoint}/json`),
    10000,
  );
  if (!response.ok) throw new Error(`CDP target discovery returned ${response.status}`);
  return response.json();
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

function writeEvidence(capability, recovery, journey) {
  if (!evidencePath) return;
  if (!targetFactsPath) throw new Error("Android RemoteBrowser evidence requires target facts");
  const target = JSON.parse(readFileSync(targetFactsPath, "utf8"));
  const evidence = {
    schema: 1,
    producer: "smoke-mobile-android-remote-browser",
    mode: "remote-browser",
    target,
    webcrypto: capability,
    journey,
    recovery,
  };
  mkdirSync(dirname(evidencePath), { recursive: true });
  writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
}

async function main() {
  if (!cdpEndpoint || !remoteOrigin || !username || !password || !adb || !serial) {
    throw new Error("CDP endpoint, remote origin, credentials, adb, and serial are required");
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
  await loginAndroidRemote(page, username, password, waitUntil);
  const stamp = Date.now();
  await createAndroidDocument(
    page,
    `android-remote-${stamp}.md`,
    `Android RemoteBrowser smoke ${stamp}`,
    {
      waitUntil,
      inputEditorText: async (content) => dispatchWebViewText(page, content),
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
  await loginAndroidRemote(page, username, password, waitUntil);
  await assertRemoteBridgeIsolation(page);
  assert.equal(new URL(await page.call(() => location.href)).origin, new URL(remoteOrigin).origin);
  const remoteScope = await readScope(page);
  assert.equal(remoteScope.status, "ready");
  assert.ok(remoteScope.repoId, "RemoteBrowser must expose a repo-scoped ready handshake");
  assert.ok(Number.isInteger(remoteScope.scopeNonce) && remoteScope.scopeNonce > 0);

  const ipcRequests = observations.requests.filter((url) => url.includes("ipc.localhost"));
  const ipcCspErrors = observations.consoleErrors.filter((message) =>
    /ipc\.localhost|content security policy|refused to connect/i.test(message));
  assert.deepEqual(ipcRequests, []);
  assert.deepEqual(ipcCspErrors, []);

  const remotePid = appPid();
  assert.ok(remotePid, "RemoteBrowser process must remain running before local recovery");
  const preRecoveryLog = adbOutput("logcat", "-d");
  assert.doesNotMatch(
    preRecoveryLog,
    /deve_mobile .*LocalBackend/,
    "preference-driven RemoteBrowser must not start LocalBackend before native intent",
  );
  tapNativeRecoveryControl();
  await page.close().catch(() => {});
  const remoteSurfaceRetired = await waitUntil("RemoteBrowser CDP target retirement", async () => {
    const targets = await listCdpTargets();
    return !targets.some((target) => {
      try {
        return target.type === "page"
          && new URL(target.url).origin === new URL(remoteOrigin).origin;
      } catch {
        return false;
      }
    });
  }, 30000);
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
  const localScope = await waitUntil("fresh LocalBackend repo scope", async () => {
    const scope = await readScope(page);
    return scope.status === "ready"
      && scope.repoId
      && Number.isInteger(scope.scopeNonce)
      && scope.scopeNonce > 0
      ? scope
      : null;
  }, 60000);
  const transition = await nativeInvoke(page, "native_backend_get_recovery_state");
  assert.equal(transition.phase, "local_window_created");
  assert.equal(transition.remoteSurfaceRetired, true);
  assert.equal(transition.preferenceCommittedAfterRemoteRetirement, true);
  assert.equal(transition.localPluginsRegisteredAfterRemoteRetirement, true);
  assert.equal(transition.supervisorManaged, true);
  assert.equal(transition.localWindowCreated, true);
  assert.equal(transition.activeRuntimeOwners, 1);
  assert.equal(transition.lastError, null);
  const localPid = appPid();
  assert.equal(localPid, remotePid, "mobile recovery must not orphan or replace the app process");
  assert.notEqual(localRuntime.origin, remoteRuntime.origin);
  const authorityTupleChanged = remoteRuntime.origin !== localRuntime.origin
    && `${remoteScope.repoId}:${remoteScope.scopeNonce}`
      !== `${localScope.repoId}:${localScope.scopeNonce}`;
  assert.equal(authorityTupleChanged, true, "remote authority tuple must not be reused locally");

  await nativeInvoke(page, "native_backend_debug_request_exit").catch((error) => {
    if (!/CDP socket closed|Inspected target navigated or closed/i.test(String(error))) throw error;
  });
  await page.close().catch(() => {});
  const processExitedAfterGracefulShutdown = await waitUntil(
    "Mobile LocalBackend graceful process exit",
    () => appPid() === "",
    30000,
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
      repoId: localScope.repoId,
      scopeNonce: localScope.scopeNonce,
    },
    remoteTargetRetired: remoteSurfaceRetired,
    authorityTupleChanged,
    appPidStable: localPid === remotePid,
    processExitedAfterGracefulShutdown,
  };
  const journey = {
    loginOrNativeSession: true,
    edit: true,
    commitHistory: true,
    backgroundResume: true,
    zeroNativeIpc: ipcRequests.length === 0 && ipcCspErrors.length === 0,
    nativeLocalRecovery: transition.phase === "local_window_created",
    remoteSurfaceDestroyedBeforeLocalIpc: remoteSurfaceRetired
      && transition.remoteSurfaceRetired
      && transition.localPluginsRegisteredAfterRemoteRetirement,
    freshLocalEndpointSessionScope: Boolean(localScope.repoId)
      && localScope.scopeNonce > 0
      && service.session_generation === 1,
    remoteAuthorityNotReused: authorityTupleChanged,
    noOrphanEmbeddedRuntime: transition.activeRuntimeOwners === 1
      && processExitedAfterGracefulShutdown,
    writableLifecycleComplete: true,
  };
  assert.ok(Object.values(journey).every((value) => value === true));
  writeEvidence(capability, recovery, journey);
  console.log("mobile-android-remote-browser: ok");
}

main().then(
  () => process.exit(0),
  (error) => {
    console.error(`mobile-android-remote-browser: ${error.stack ?? error.message}`);
    process.exit(1);
  },
);

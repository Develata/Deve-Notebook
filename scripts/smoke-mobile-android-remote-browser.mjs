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

function writeEvidence(capability) {
  if (!evidencePath) return;
  if (!targetFactsPath) throw new Error("Android RemoteBrowser evidence requires target facts");
  const target = JSON.parse(readFileSync(targetFactsPath, "utf8"));
  const evidence = {
    schema: 1,
    producer: "smoke-mobile-android-remote-browser",
    mode: "remote-browser",
    target,
    webcrypto: capability,
    journey: {
      loginOrNativeSession: true,
      edit: true,
      commitHistory: true,
      backgroundResume: true,
      zeroNativeIpc: true,
      writableLifecycleComplete: true,
    },
  };
  mkdirSync(dirname(evidencePath), { recursive: true });
  writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
}

async function main() {
  if (!cdpEndpoint || !remoteOrigin || !username || !password || !adb || !serial) {
    throw new Error("CDP endpoint, remote origin, credentials, adb, and serial are required");
  }
  assert.equal(new URL(remoteOrigin).protocol, "https:");
  let page = await attachRemotePage();
  const requests = [];
  const consoleErrors = [];
  page.on("Network.requestWillBeSent", ({ request }) => requests.push(request?.url ?? ""));
  page.on("Log.entryAdded", ({ entry }) => {
    if (entry?.level === "error") consoleErrors.push(entry.text ?? "");
  });
  await page.send("Network.enable");
  await page.send("Log.enable");
  await page.send("Page.reload", { ignoreCache: true });
  await waitUntil("RemoteBrowser DOM reload", () => page.call(() =>
    Boolean(document.querySelector("[data-deve-sync-status]"))), 30000);
  await page.evaluate(`globalThis.__deveVisibleElement = ${visibleElement.toString()}`);

  const bridge = await page.call(() => ({
    capability: globalThis.__DEVE_NATIVE_BOOTSTRAP?.capabilities
      ?.backend_preference_control === true,
    facade: Boolean(globalThis.__deveWebBridge?.get?.("__DEVE_NATIVE_BACKEND_CONFIG__")),
    directFacade: Boolean(globalThis.__DEVE_NATIVE_BACKEND_CONFIG__),
  }));
  assert.deepEqual(bridge, { capability: false, facade: false, directFacade: false });
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

  adbCommand("shell", "input", "keyevent", "3");
  await page.close();
  await delay(500);
  adbCommand("shell", "monkey", "-p", appId, "-c", "android.intent.category.LAUNCHER", "1");
  page = await attachRemotePage();
  await loginAndroidRemote(page, username, password, waitUntil);
  assert.equal(new URL(await page.call(() => location.href)).origin, new URL(remoteOrigin).origin);

  const ipcRequests = requests.filter((url) => url.includes("ipc.localhost"));
  const ipcCspErrors = consoleErrors.filter((message) =>
    /ipc\.localhost|content security policy|refused to connect/i.test(message));
  assert.deepEqual(ipcRequests, []);
  assert.deepEqual(ipcCspErrors, []);
  writeEvidence(capability);
  await page.close();
  console.log("mobile-android-remote-browser: ok");
}

main().then(
  () => process.exit(0),
  (error) => {
    console.error(`mobile-android-remote-browser: ${error.stack ?? error.message}`);
    process.exit(1);
  },
);

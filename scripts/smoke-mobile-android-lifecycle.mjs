import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { probeWebCryptoEd25519 } from "./lib/webcrypto-capability.mjs";
import { evaluateWritableProbeExpectation } from "./lib/android-target-capability.mjs";
import {
  findStableAppPage,
  visibleElement,
} from "./lib/android-webview-cdp.mjs";
import {
  clickWebViewPoint,
  focusEditor,
  proveSameBreakpointKeyboardResize,
  readPendingAckCount,
  typeEditor,
} from "./lib/mobile-webview-interaction.mjs";
import {
  commitAndroidChange,
  createAndroidDocument,
  dispatchWebViewText,
  waitForWritableEditor as waitForWritableAndroidEditor,
} from "./lib/android-business-flow.mjs";
import {
  discardAndResumeWebSocketDelivery,
  inspectWebSocketDeliveryGate,
  installWebSocketDeliveryGate,
  pauseWebSocketDelivery,
} from "./lib/websocket-delivery-gate.mjs";

const timeoutMs = Number(process.env.DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_MS ?? "90000");
const cdpEndpoint = process.env.DEVE_MOBILE_ANDROID_CDP_ENDPOINT;
const adb = process.env.DEVE_MOBILE_ANDROID_ADB_BIN;
const serial = process.env.DEVE_MOBILE_ANDROID_SERIAL;
const appId = process.env.DEVE_MOBILE_ANDROID_APP_ID ?? "dev.deve.notebook.mobile";
const expectWritable = process.env.DEVE_MOBILE_ANDROID_EXPECT_WRITABLE !== "0";
const targetFactsPath = process.env.DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH;
const evidencePath = process.env.DEVE_MOBILE_ANDROID_EVIDENCE_PATH;
const harnessDeadline = Date.now() + timeoutMs;

function remainingMs() {
  const remaining = harnessDeadline - Date.now();
  if (remaining <= 0) throw new Error("Android lifecycle harness deadline exhausted");
  return remaining;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
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
  const deadline = Math.min(Date.now() + timeout, harnessDeadline);
  let lastError;
  while (Date.now() < deadline) {
    try {
      const value = await withDeadline(
        `${label} predicate`,
        Promise.resolve().then(predicate),
        Math.max(1, deadline - Date.now()),
      );
      if (value) return value;
    } catch (error) {
      lastError = error;
    }
    await delay(Math.min(250, Math.max(1, deadline - Date.now())));
  }
  throw new Error(`timeout waiting for ${label}${lastError ? `: ${lastError.message}` : ""}`);
}

function adbCommand(...args) {
  execFileSync(adb, ["-s", serial, ...args], {
    stdio: "inherit",
    timeout: remainingMs(),
  });
}

async function focusWebViewEditorAtPoint(point, page) {
  let lastFocusError;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    await clickWebViewPoint(page, point);
    await delay(250);
    await waitForWritableEditor(page);
    try {
      await focusEditor(page);
      lastFocusError = null;
      break;
    } catch (error) {
      lastFocusError = error;
      await delay(250);
    }
  }
  if (lastFocusError) throw lastFocusError;
}

async function inputAndroidEditorText(content, point, page) {
  await focusWebViewEditorAtPoint(point, page);
  // Android WebView can report DOM focus before its input connection has
  // settled after an adb tap. Let it settle, then dispatch keyboard characters.
  await delay(300);
  await dispatchWebViewText(page, content);
}

async function waitForReady(page) {
  await waitUntil("ready sync status", () => page.call(() =>
    document.querySelector("[data-deve-sync-status]")?.getAttribute("data-deve-sync-status") === "ready"), 60000);
}

function readIdentityCapabilityUiState(page, blocker) {
  return page.call((expectedBlocker) => {
    const status = document.querySelector("[data-deve-sync-status]")
      ?.getAttribute("data-deve-sync-status") ?? null;
    const body = document.body?.textContent ?? "";
    const reasonVisible = expectedBlocker === "ed25519_unavailable"
      ? body.includes("WebCrypto Ed25519") && body.includes("Android System WebView")
      : expectedBlocker === "webcrypto_unavailable"
        ? body.includes("Browser cryptography") || body.includes("浏览器加密能力")
        : body.includes("Browser identity capability check failed")
          || body.includes("浏览器身份能力探测失败");
    const editors = [...document.querySelectorAll("[data-deve-editor-host=true]")];
    const editorsReadOnly = editors.length > 0
      && editors.every((host) => host.getAttribute("data-deve-editor-readonly") === "true");
    const dashboardCreate = document.querySelector(
      "[data-deve-dashboard-card='quick-actions'] button[data-deve-mobile-touch-target='dashboard_quick_actions']",
    );
    const emptyRepoMutationLocked = editors.length === 0
      && dashboardCreate instanceof HTMLButtonElement
      && dashboardCreate.disabled
      && document.querySelector("[data-deve-new-doc-button=true]") === null;
    return {
      ready: status === "read-only"
        && reasonVisible
        && (editorsReadOnly || emptyRepoMutationLocked),
      status,
      reasonVisible,
      editorCount: editors.length,
      editorReadOnlyValues: editors.map((host) => host.getAttribute("data-deve-editor-readonly")),
      dashboardCreateDisabled: dashboardCreate instanceof HTMLButtonElement
        ? dashboardCreate.disabled
        : null,
      sidebarCreateVisible: document.querySelector("[data-deve-new-doc-button=true]") !== null,
      bodyExcerpt: body.replace(/\s+/g, " ").trim().slice(0, 800),
    };
  }, blocker);
}

async function verifyIdentityCapability(page) {
  const capability = await withDeadline(
    "non-extractable WebCrypto Ed25519 capability probe",
    page.call(probeWebCryptoEd25519),
    30000,
  );
  console.log(`mobile-android-lifecycle: WebCrypto capability ${JSON.stringify(capability)}`);
  if (!capability.writable) {
    try {
      await waitUntil("storage-limited read-only identity state", async () =>
        (await readIdentityCapabilityUiState(page, capability.blocker)).ready);
    } catch (error) {
      const observed = await readIdentityCapabilityUiState(page, capability.blocker);
      throw new Error(`${error.message}; observed=${JSON.stringify(observed)}`);
    }
  }
  evaluateWritableProbeExpectation(expectWritable, capability);
  return capability;
}

async function proveReadonlyMutationRejected(page) {
  const before = await page.call(() => {
    const host = globalThis.__deveVisibleElement("[data-deve-editor-host=true]");
    const content = globalThis.__deveVisibleElement(".cm-content");
    const createButton = document.querySelector(
      "[data-deve-dashboard-card='quick-actions'] button[data-deve-mobile-touch-target='dashboard_quick_actions']",
    );
    if (host && content) content.focus();
    return {
      hasEditor: Boolean(host && content),
      text: host && content ? window.getEditorContent?.() ?? content.textContent ?? "" : null,
      pending: document.querySelector("[data-deve-mobile-pending-ack-count]")
        ?.getAttribute("data-deve-mobile-pending-ack-count")
        ?? document.querySelector("[data-deve-pending-ack-count]")
          ?.getAttribute("data-deve-pending-ack-count")
        ?? "0",
      readOnly: host?.getAttribute("data-deve-editor-readonly") ?? null,
      contentEditable: content?.getAttribute("contenteditable") ?? null,
      docCount: document.querySelector("[data-deve-dashboard-storage-doc-count]")
        ?.getAttribute("data-deve-dashboard-storage-doc-count") ?? null,
      createDisabled: createButton instanceof HTMLButtonElement ? createButton.disabled : null,
    };
  });
  if (before.hasEditor) {
    assert.equal(before.readOnly, "true");
    assert.notEqual(before.contentEditable, "true");
    await page.send("Input.insertText", { text: "MUST_NOT_APPLY" });
  } else {
    assert.equal(before.createDisabled, true, "empty read-only repo exposed document creation");
    await page.call(() => document.querySelector(
      "[data-deve-dashboard-card='quick-actions'] button[data-deve-mobile-touch-target='dashboard_quick_actions']",
    )?.click());
  }
  await delay(250);
  const after = await editorDiagnostics(page);
  if (before.hasEditor) {
    assert.equal(after.bridgeContent ?? after.domContent, before.text, "read-only editor accepted input");
  } else {
    const emptyRepoAfter = await page.call(() => ({
      docCount: document.querySelector("[data-deve-dashboard-storage-doc-count]")
        ?.getAttribute("data-deve-dashboard-storage-doc-count") ?? null,
      editorCount: document.querySelectorAll("[data-deve-editor-host=true]").length,
      sidebarCreateVisible: document.querySelector("[data-deve-new-doc-button=true]") !== null,
    }));
    assert.equal(emptyRepoAfter.docCount, before.docCount, "read-only create attempt changed doc count");
    assert.equal(emptyRepoAfter.editorCount, 0, "read-only create attempt opened an editor");
    assert.equal(emptyRepoAfter.sidebarCreateVisible, false, "read-only sidebar exposed document creation");
  }
  assert.equal(
    String(after.pending ?? "0"),
    String(before.pending),
    "read-only mutation attempt changed pending state",
  );
  return { editorInputRejected: before.hasEditor, emptyRepoCreateRejected: !before.hasEditor };
}

function writeAndroidEvidence(capability, journey) {
  if (!evidencePath) return;
  if (!targetFactsPath) throw new Error("Android evidence requires target facts path");
  const target = JSON.parse(readFileSync(targetFactsPath, "utf8"));
  const evidence = {
    schema: 1,
    producer: "smoke-mobile-android-lifecycle",
    mode: "local-backend",
    target,
    webcrypto: capability,
    journey,
  };
  mkdirSync(dirname(evidencePath), { recursive: true });
  writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
}

async function waitForWritableEditor(page, timeout = 30000) {
  return waitForWritableAndroidEditor(page, waitUntil, timeout);
}

async function reloadWithWebSocketDeliveryGate(page) {
  await installWebSocketDeliveryGate(page);
  await page.send("Page.reload", { ignoreCache: true });
  await waitUntil("reloaded Android app DOM", () => page.call(() =>
    Boolean(document.querySelector("[data-deve-sync-status]"))), 30000);
  await page.evaluate(`globalThis.__deveVisibleElement = ${visibleElement.toString()}`);
}

function editorContent(page) {
  return page.call(() => window.getEditorContent?.() ?? null);
}

function editorDiagnostics(page) {
  return page.call(() => ({
    bridgeContent: window.getEditorContent?.() ?? null,
    domContent: globalThis.__deveVisibleElement(".cm-content")?.textContent ?? null,
    readOnly: globalThis.__deveVisibleElement("[data-deve-editor-host=true]")
      ?.getAttribute("data-deve-editor-readonly") ?? null,
    pending: document.querySelector("[data-deve-mobile-pending-ack-count]")
      ?.getAttribute("data-deve-mobile-pending-ack-count")
      ?? document.querySelector("[data-deve-pending-ack-count]")
        ?.getAttribute("data-deve-pending-ack-count")
      ?? null,
    syncStatus: document.querySelector("[data-deve-sync-status]")
      ?.getAttribute("data-deve-sync-status") ?? null,
  }));
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
      return {
        ok: false,
        error: {
          name: error?.name ?? null,
          message: error?.message ?? null,
          detail: String(error),
        },
      };
    }
  }, command, args);
  if (!outcome.ok) {
    throw new Error(`native command ${command} failed: ${JSON.stringify(outcome.error)}`);
  }
  return outcome.value;
}

async function createDocument(page, path, content) {
  console.log(`mobile-android-lifecycle: creating document ${path}`);
  return createAndroidDocument(page, path, content, {
    waitUntil,
    inputEditorText: inputAndroidEditorText,
  });
}

function commit(page, message) {
  return commitAndroidChange(page, message, { waitUntil, delay });
}

async function main() {
  if (!cdpEndpoint || !adb || !serial) {
    throw new Error("CDP endpoint, adb path, and emulator serial are required");
  }
  const page = await findStableAppPage({ cdpEndpoint, withDeadline, waitUntil });
  await reloadWithWebSocketDeliveryGate(page);

  console.log("mobile-android-lifecycle: waiting for native session");
  const identityCapability = await verifyIdentityCapability(page);
  if (!expectWritable) {
    assert.equal(identityCapability.writable, false);
    const readonlyProof = await proveReadonlyMutationRejected(page);
    writeAndroidEvidence(identityCapability, {
      readonlyMutationRejected: true,
      ...readonlyProof,
      writableLifecycleComplete: false,
    });
    console.log(
      `mobile-android-lifecycle: readonly negative evidence accepted blocker=${identityCapability.blocker}`,
    );
    try {
      await nativeInvoke(page, "native_backend_debug_request_exit");
    } catch (error) {
      throw new Error(`native graceful exit request failed: ${error.message}`);
    }
    await page.close();
    console.log("mobile-android-lifecycle: readonly-negative ok");
    return;
  }
  console.log("mobile-android-lifecycle: WebCrypto capability accepted");
  await waitForReady(page);
  console.log("mobile-android-lifecycle: native session ready");
  assert.equal(await page.call(() => document.querySelectorAll("#login-username").length), 0,
    "native session must bypass login");

  const stamp = Date.now();
  const initial = `Android lifecycle smoke ${stamp}`;
  await createDocument(page, `android-lifecycle-${stamp}.md`, initial);
  await commit(page, `android lifecycle initial ${stamp}`);
  await waitForWritableEditor(page);
  await waitUntil("committed document projection restored", () => page.call((expected) =>
    window.getEditorContent?.()?.includes(expected) ?? false,
  initial));
  const serviceBeforeSuspend = await nativeInvoke(page, "native_backend_get_service_state");
  assert.equal(serviceBeforeSuspend?.backend_running, true, "embedded transport must be running");

  const pendingText = `-pending-across-restart-${stamp}`;
  adbCommand("shell", "input", "keyevent", "111");
  const keyboardResize = await proveSameBreakpointKeyboardResize(page, {
    waitUntil,
    activateKeyboard: focusWebViewEditorAtPoint,
  });
  console.log(
    `mobile-android-lifecycle: keyboard resize ${JSON.stringify(keyboardResize)}`,
  );

  await pauseWebSocketDelivery(page);
  await dispatchWebViewText(page, pendingText);
  const contentBeforeSuspend = await waitUntil("pending editor input", () => page.call(
    (expected) => {
      const observed = window.getEditorContent?.();
      return observed?.includes(expected) ? observed : null;
    },
    pendingText,
  ));
  await waitUntil("nonzero pending before suspend", async () => (await readPendingAckCount(page)) > 0);
  const deliveryGate = await waitUntil("buffered outbound write before suspend", async () => {
    const gate = await inspectWebSocketDeliveryGate(page);
    return gate.pending > 0 ? gate : null;
  }, 10000);
  console.log(`mobile-android-lifecycle: pending delivery gate ${JSON.stringify(deliveryGate)}`);
  assert.ok(deliveryGate.pending > 0, "lifecycle smoke must buffer a real outbound write");
  const pendingBeforeSuspend = await readPendingAckCount(page);
  assert.ok(pendingBeforeSuspend > 0, "lifecycle smoke must preserve a real pending overlay");

  console.log("mobile-android-lifecycle: backgrounding app");
  adbCommand("shell", "input", "keyevent", "3");
  await waitUntil("suspended editor read-only", () => page.call(() =>
    document.querySelector("[data-deve-editor-host=true]")?.getAttribute("data-deve-editor-readonly") === "true"));
  const suspendedDiagnostics = await editorDiagnostics(page);
  console.log(`mobile-android-lifecycle: suspended editor ${JSON.stringify(suspendedDiagnostics)}`);
  assert.equal(
    Number(suspendedDiagnostics.pending),
    pendingBeforeSuspend,
    "suspend must retain the pending overlay signal before blocked input",
  );
  const suspendedProjectionContent = suspendedDiagnostics.bridgeContent;
  const blockedText = `-MUST-NOT-APPLY-${stamp}`;
  await focusEditor(page, { writable: false });
  await page.send("Input.insertText", { text: blockedText });
  const blockedDiagnostics = await editorDiagnostics(page);
  console.log(`mobile-android-lifecycle: blocked input editor ${JSON.stringify(blockedDiagnostics)}`);
  assert.equal(
    blockedDiagnostics.bridgeContent,
    suspendedProjectionContent,
    "suspended editor must reject writes",
  );
  assert.equal(
    await readPendingAckCount(page),
    pendingBeforeSuspend,
    "suspend must preserve pending count",
  );

  console.log("mobile-android-lifecycle: stopping current transport generation");
  await nativeInvoke(page, "native_backend_debug_stop_transport");
  await waitUntil("transport generation stopped", async () => {
    const state = await nativeInvoke(page, "native_backend_get_service_state");
    return state?.backend_running === false;
  });
  const discarded = await discardAndResumeWebSocketDelivery(page);
  assert.ok(discarded.released > 0, "stale transport writes must be discarded before resume");

  console.log("mobile-android-lifecycle: resuming app");
  adbCommand("shell", "monkey", "-p", appId, "-c", "android.intent.category.LAUNCHER", "1");
  const serviceAfterResume = await waitUntil("replacement transport generation", async () => {
    const state = await nativeInvoke(page, "native_backend_get_service_state");
    return state?.backend_running === true
      && state.session_generation === serviceBeforeSuspend.session_generation + 1
      && state.endpoint !== serviceBeforeSuspend.endpoint
      ? state
      : null;
  }, 120000).catch(async (error) => {
    const state = await nativeInvoke(page, "native_backend_get_service_state").catch(
      (nativeError) => ({ error: nativeError.message }),
    );
    throw new Error(`${error.message}; native=${JSON.stringify(state)}`);
  });
  assert.equal(serviceAfterResume.service_state, "endpoint_session_ready");
  await waitForWritableEditor(page, 60000).catch(async (error) => {
    const diagnostics = await page.call(() => ({
      url: location.href,
      readyState: document.readyState,
      syncStatus: document.querySelector("[data-deve-sync-status]")
        ?.getAttribute("data-deve-sync-status") ?? null,
      pending: document.querySelector("[data-deve-mobile-pending-ack-count]")
        ?.getAttribute("data-deve-mobile-pending-ack-count") ?? null,
      bootPanelDisplay: getComputedStyle(document.querySelector("#boot-panel")).display,
      bootPanelDetail: document.querySelector("#boot-panel-detail")?.textContent ?? null,
      bodyText: (document.body?.textContent ?? "").replace(/\s+/g, " ").slice(0, 800),
    }));
    const nativeState = await nativeInvoke(page, "native_backend_get_service_state").catch(
      (nativeError) => ({ error: nativeError.message }),
    );
    const gate = await inspectWebSocketDeliveryGate(page).catch(
      (gateError) => ({ error: gateError.message }),
    );
    throw new Error(
      `${error.message}; page=${JSON.stringify(diagnostics)}; native=${JSON.stringify(nativeState)}; gate=${JSON.stringify(gate)}`,
    );
  });
  await waitUntil("replacement generation pending replay", async () =>
    (await editorContent(page)) === contentBeforeSuspend, 30000).catch(async (error) => {
    const diagnostics = await editorDiagnostics(page).catch(
      (diagnosticError) => ({ error: diagnosticError.message }),
    );
    throw new Error(
      `${error.message}; expected=${JSON.stringify(contentBeforeSuspend)}; editor=${JSON.stringify(diagnostics)}`,
    );
  });
  await waitUntil("preserved pending replay ack", async () => (await readPendingAckCount(page)) === 0);
  const resumedText = `-resumed-${stamp}`;
  await typeEditor(page, resumedText, waitUntil, inputAndroidEditorText);
  await waitUntil("resumed edit ack", async () => (await readPendingAckCount(page)) === 0);
  await commit(page, `android lifecycle resumed ${stamp}`);

  writeAndroidEvidence(identityCapability, {
    loginOrNativeSession: true,
    edit: true,
    commitHistory: true,
    backgroundResume: true,
    staleScopeRejected: true,
    pendingPreserved: true,
    writableLifecycleComplete: true,
  });

  console.log("mobile-android-lifecycle: requesting graceful native exit");
  try {
    await nativeInvoke(page, "native_backend_debug_request_exit");
  } catch (error) {
    throw new Error(`native graceful exit request failed: ${error.message}`);
  }
  await page.close();
  console.log("mobile-android-lifecycle: ok");
}

main().then(
  () => process.exit(0),
  (error) => {
    console.error(`mobile-android-lifecycle: ${error.stack ?? error.message}`);
    process.exit(1);
  },
);

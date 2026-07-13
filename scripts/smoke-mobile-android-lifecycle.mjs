import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { probeWebCryptoEd25519 } from "./lib/webcrypto-capability.mjs";
import {
  closeMobileSidebar,
  focusEditor,
  openMobileSidebarView,
  typeEditor,
} from "./lib/mobile-webview-interaction.mjs";

const timeoutMs = Number(process.env.DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_MS ?? "90000");
const cdpEndpoint = process.env.DEVE_MOBILE_ANDROID_CDP_ENDPOINT;
const adb = process.env.DEVE_MOBILE_ANDROID_ADB_BIN;
const serial = process.env.DEVE_MOBILE_ANDROID_SERIAL;
const appId = process.env.DEVE_MOBILE_ANDROID_APP_ID ?? "dev.deve.notebook.mobile";
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

class CdpPage {
  constructor(socket) {
    this.socket = socket;
    this.nextId = 1;
    this.pending = new Map();
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(String(event.data));
      if (message.id === undefined) return;
      const waiter = this.pending.get(message.id);
      if (!waiter) return;
      this.pending.delete(message.id);
      if (message.error) waiter.reject(new Error(`${waiter.method}: ${message.error.message}`));
      else waiter.resolve(message.result ?? {});
    });
    socket.addEventListener("close", () => {
      for (const waiter of this.pending.values()) {
        waiter.reject(new Error(`CDP socket closed during ${waiter.method}`));
      }
      this.pending.clear();
    });
  }

  static async connect(webSocketDebuggerUrl) {
    const socket = new WebSocket(webSocketDebuggerUrl);
    try {
      await withDeadline("Android WebView CDP socket open", new Promise((resolve, reject) => {
        socket.addEventListener("open", resolve, { once: true });
        socket.addEventListener("error", () => reject(new Error("Android WebView CDP socket failed")), { once: true });
      }), 10000);
      const page = new CdpPage(socket);
      await page.send("Runtime.enable");
      return page;
    } catch (error) {
      try {
        socket.close();
      } catch {}
      throw error;
    }
  }

  send(method, params = {}) {
    const id = this.nextId++;
    return withDeadline(method, new Promise((resolve, reject) => {
      this.pending.set(id, { method, resolve, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
    }));
  }

  async evaluate(expression) {
    const response = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
    });
    if (response.exceptionDetails) {
      const description = response.exceptionDetails.exception?.description
        ?? response.exceptionDetails.text
        ?? "JavaScript evaluation failed";
      throw new Error(description);
    }
    return response.result?.value;
  }

  call(fn, ...args) {
    return this.evaluate(`(${fn.toString()})(...${JSON.stringify(args)})`);
  }

  async close() {
    if (this.socket.readyState === WebSocket.CLOSED) return;
    this.socket.close();
    await withDeadline("Android WebView CDP socket close", new Promise((resolve) => {
      this.socket.addEventListener("close", resolve, { once: true });
    }), 2000).catch(() => {});
  }
}

async function listTargets() {
  const response = await withDeadline("Android WebView target discovery", fetch(`${cdpEndpoint}/json`), 10000);
  if (!response.ok) throw new Error(`CDP target discovery returned ${response.status}`);
  return response.json();
}

async function findAppPage() {
  const targets = await listTargets();
  const discoveredTargets = targets.map(({ type, title, url }) => ({ type, title, url }));
  const target = targets.find((candidate) =>
    candidate.webSocketDebuggerUrl
    && candidate.type === "page"
    && candidate.url === "http://tauri.localhost/");
  if (!target) {
    throw new Error(`Android WebView target unavailable; targets=${JSON.stringify(discoveredTargets)}`);
  }
  console.log(`mobile-android-lifecycle: attaching page CDP ${target.title}`);
  let page;
  try {
    page = await CdpPage.connect(target.webSocketDebuggerUrl);
    console.log("mobile-android-lifecycle: page CDP attached");
    await waitUntil("Android app DOM", () => page.call(() =>
      Boolean(document.querySelector("[data-deve-sync-status]"))), 10000);
    await page.evaluate(`globalThis.__deveVisibleElement = ${visibleElement.toString()}`);
  } catch (error) {
    const diagnostics = page
      ? await page.call(() => ({
        url: location.href,
        readyState: document.readyState,
        title: document.title,
        bodyText: (document.body?.textContent ?? "").slice(0, 500),
        bodyHtml: (document.body?.innerHTML ?? "").slice(0, 500),
      })).catch((diagnosticError) => ({ diagnosticError: diagnosticError.message }))
      : { diagnosticError: "page unavailable before Runtime.enable" };
    await page?.close();
    throw new Error(`${error.message}; page=${JSON.stringify(diagnostics)}`);
  }
  return page;
}

async function findStableAppPage() {
  return waitUntil("stable Android WebView page", async () => {
    try {
      return await findAppPage();
    } catch (error) {
      const message = String(error?.message ?? error);
      if (message.includes("Inspected target navigated or closed")
        || message.includes("CDP socket closed")
        || message.includes("Android WebView target unavailable")) {
        console.log(`mobile-android-lifecycle: retrying page CDP after navigation: ${message}`);
        return null;
      }
      throw error;
    }
  }, 60000);
}

function visibleElement(selector) {
  return [...document.querySelectorAll(selector)].find((element) => {
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
  }) ?? null;
}

async function waitForReady(page) {
  await waitUntil("ready sync status", () => page.call(() =>
    document.querySelector("[data-deve-sync-status]")?.getAttribute("data-deve-sync-status") === "ready"), 60000);
}

async function assertWritableIdentityCapability(page) {
  const capability = await withDeadline(
    "non-extractable WebCrypto Ed25519 capability probe",
    page.call(probeWebCryptoEd25519),
    30000,
  );
  console.log(`mobile-android-lifecycle: WebCrypto capability ${JSON.stringify(capability)}`);
  if (!capability.writable) {
    await waitUntil("storage-limited read-only identity state", () => page.call((blocker) => {
      const status = document.querySelector("[data-deve-sync-status]")
        ?.getAttribute("data-deve-sync-status");
      const body = document.body?.textContent ?? "";
      const reasonVisible = blocker === "ed25519_unavailable"
        ? body.includes("WebCrypto Ed25519") && body.includes("Android System WebView")
        : blocker === "webcrypto_unavailable"
          ? body.includes("Browser cryptography") || body.includes("浏览器加密能力")
          : body.includes("Browser identity capability check failed")
            || body.includes("浏览器身份能力探测失败");
      const editorsReadOnly = [...document.querySelectorAll("[data-deve-editor-host=true]")]
        .every((host) => host.getAttribute("data-deve-editor-readonly") === "true");
      return status === "read-only" && reasonVisible && editorsReadOnly;
    }, capability.blocker));
    throw new Error(
      `Android System WebView WebCrypto capability blocked writable lifecycle: ${capability.blocker}; userAgent=${capability.userAgent}`,
    );
  }
}

async function waitForWritableEditor(page) {
  await waitUntil("visible Android editor", () => page.call(() =>
    Boolean(globalThis.__deveVisibleElement("[data-deve-editor-host=true]"))));
  await waitUntil("writable Android editor", () => page.call(() =>
    globalThis.__deveVisibleElement("[data-deve-editor-host=true]")
      ?.getAttribute("data-deve-editor-readonly") === "false"));
}

function editorContent(page) {
  return page.call(() => window.getEditorContent?.() ?? null);
}

async function pendingCount(page) {
  return Number(await page.call(() =>
    document.querySelector("[data-deve-pending-ack-count]")?.getAttribute("data-deve-pending-ack-count")));
}

function nativeInvoke(page, command, args = {}) {
  return page.call(async (invokeCommand, invokeArgs) => {
    const invoke = window.__TAURI_INTERNALS__?.invoke;
    if (typeof invoke !== "function") throw new Error("Tauri invoke bridge unavailable");
    return invoke(invokeCommand, invokeArgs);
  }, command, args);
}

async function setNetworkLatency(page, latency) {
  await page.send("Network.enable");
  await page.send("Network.emulateNetworkConditions", {
    offline: false,
    latency,
    downloadThroughput: -1,
    uploadThroughput: -1,
  });
}

async function click(page, selector) {
  const clicked = await page.call((target) => {
    const element = globalThis.__deveVisibleElement(target);
    if (!element) return false;
    element.click();
    return true;
  }, selector);
  if (!clicked) throw new Error(`visible click target not found: ${selector}`);
}

async function fill(page, selector, value) {
  const filled = await page.call((target, nextValue) => {
    const element = globalThis.__deveVisibleElement(target);
    if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) return false;
    const prototype = element instanceof HTMLTextAreaElement
      ? HTMLTextAreaElement.prototype
      : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(prototype, "value").set.call(element, nextValue);
    element.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: nextValue }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  }, selector, value);
  if (!filled) throw new Error(`visible form field not found: ${selector}`);
}

async function createDocument(page, path, content) {
  console.log(`mobile-android-lifecycle: creating document ${path}`);
  const mobile = await page.call(() =>
    Boolean(globalThis.__deveVisibleElement('[data-deve-layout-mode="mobile"]')));
  if (mobile) await openMobileSidebarView(page, "explorer", { click, waitUntil });
  await click(page, "[data-deve-new-doc-button=true]");
  await waitUntil("new document input", () => page.call(() =>
    Boolean(globalThis.__deveVisibleElement("[data-deve-search-input=true]"))));
  await fill(page, "[data-deve-search-input=true]", `+${path}`);
  await waitUntil("create document action", () => page.call(() =>
    Boolean(globalThis.__deveVisibleElement('[data-deve-search-result-action="create-doc"]'))));
  await click(page, '[data-deve-search-result-action="create-doc"]');
  console.log("mobile-android-lifecycle: document create intent submitted");
  await waitForWritableEditor(page);
  console.log("mobile-android-lifecycle: editor writable");
  await waitUntil("editor bridge", () => page.call(() =>
    typeof window.getEditorContent === "function" && typeof window.getEditorContent() === "string"));
  console.log("mobile-android-lifecycle: editor bridge ready");
  await typeEditor(page, content, waitUntil);
  console.log("mobile-android-lifecycle: initial editor input visible");
  await waitUntil("initial edit ack", async () => (await pendingCount(page)) === 0);
  console.log("mobile-android-lifecycle: initial edit acknowledged");
}

async function openSourceControl(page) {
  const mobile = await page.call(() =>
    Boolean(globalThis.__deveVisibleElement('[data-deve-layout-mode="mobile"]')));
  if (mobile) {
    await openMobileSidebarView(page, "source_control", { click, waitUntil });
    return;
  }
  await click(page, "[data-deve-activity-more-button]");
  await waitUntil("Source Control menu item", () => page.call(() =>
    Boolean(globalThis.__deveVisibleElement(
      '[data-deve-activity-more-item="activity_more_item_source_control"]'))));
  await click(page, '[data-deve-activity-more-item="activity_more_item_source_control"]');
}

async function commit(page, message) {
  await openSourceControl(page);
  const textarea = 'textarea[name="commit-message"]';
  await waitUntil("commit message input", () => page.call((selector) =>
    Boolean(globalThis.__deveVisibleElement(selector)), textarea));
  await waitUntil("commit input enabled", () => page.call((selector) =>
    !globalThis.__deveVisibleElement(selector)?.disabled, textarea));
  await fill(page, textarea, message);
  await waitUntil("commit enabled", () => page.call(() => {
    const field = globalThis.__deveVisibleElement('textarea[name="commit-message"]');
    const panel = field?.closest("div.border-t");
    const button = panel?.querySelector("button:has(.codicon-check)");
    return Boolean(button && !button.disabled);
  }));
  const committed = await page.call(() => {
    const field = globalThis.__deveVisibleElement('textarea[name="commit-message"]');
    const button = field?.closest("div.border-t")?.querySelector("button:has(.codicon-check)");
    if (!button) return false;
    button.click();
    return true;
  });
  if (!committed) throw new Error("commit action not found");
  await waitUntil("commit complete", () => page.call(() =>
    document.querySelector('textarea[name="commit-message"]')?.value === ""));
  await closeMobileSidebar(page, { click, waitUntil });
}

async function main() {
  if (!cdpEndpoint || !adb || !serial) {
    throw new Error("CDP endpoint, adb path, and emulator serial are required");
  }
  const page = await findStableAppPage();

  console.log("mobile-android-lifecycle: waiting for native session");
  await assertWritableIdentityCapability(page);
  console.log("mobile-android-lifecycle: WebCrypto capability accepted");
  await waitForReady(page);
  console.log("mobile-android-lifecycle: native session ready");
  assert.equal(await page.call(() => document.querySelectorAll("#login-username").length), 0,
    "native session must bypass login");

  const stamp = Date.now();
  const initial = `Android lifecycle smoke ${stamp}`;
  await createDocument(page, `android-lifecycle-${stamp}.md`, initial);
  await commit(page, `android lifecycle initial ${stamp}`);
  const serviceBeforeSuspend = await nativeInvoke(page, "native_backend_get_service_state");
  assert.equal(serviceBeforeSuspend?.backend_running, true, "embedded transport must be running");

  await setNetworkLatency(page, 5000);
  const pendingText = ` pending-across-restart-${stamp}`;
  await typeEditor(page, pendingText, waitUntil);
  await waitUntil("nonzero pending before suspend", async () => (await pendingCount(page)) > 0);
  const contentBeforeSuspend = await editorContent(page);
  const pendingBeforeSuspend = await pendingCount(page);
  assert.ok(pendingBeforeSuspend > 0, "lifecycle smoke must preserve a real pending overlay");

  console.log("mobile-android-lifecycle: backgrounding app");
  adbCommand("shell", "input", "keyevent", "3");
  await waitUntil("suspended editor read-only", () => page.call(() =>
    document.querySelector("[data-deve-editor-host=true]")?.getAttribute("data-deve-editor-readonly") === "true"));
  const blockedText = ` MUST-NOT-APPLY-${stamp}`;
  await focusEditor(page);
  await page.send("Input.insertText", { text: blockedText });
  assert.equal(await editorContent(page), contentBeforeSuspend, "suspended editor must reject writes");
  assert.equal(await pendingCount(page), pendingBeforeSuspend, "suspend must preserve pending count");

  console.log("mobile-android-lifecycle: stopping current transport generation");
  await nativeInvoke(page, "native_backend_debug_stop_transport");
  await waitUntil("transport generation stopped", async () => {
    const state = await nativeInvoke(page, "native_backend_get_service_state");
    return state?.backend_running === false;
  });
  await setNetworkLatency(page, 0);

  console.log("mobile-android-lifecycle: resuming app");
  adbCommand("shell", "monkey", "-p", appId, "-c", "android.intent.category.LAUNCHER", "1");
  const serviceAfterResume = await waitUntil("replacement transport generation", async () => {
    const state = await nativeInvoke(page, "native_backend_get_service_state");
    return state?.backend_running === true
      && state.session_generation === serviceBeforeSuspend.session_generation + 1
      && state.endpoint !== serviceBeforeSuspend.endpoint
      ? state
      : null;
  });
  assert.equal(serviceAfterResume.service_state, "endpoint_session_ready");
  await waitForReady(page);
  await waitForWritableEditor(page);
  assert.equal(await editorContent(page), contentBeforeSuspend, "resume must preserve document content");
  await waitUntil("preserved pending replay ack", async () => (await pendingCount(page)) === 0);
  const resumedText = ` resumed ${stamp}`;
  await typeEditor(page, resumedText, waitUntil);
  await waitUntil("resumed edit ack", async () => (await pendingCount(page)) === 0);
  await commit(page, `android lifecycle resumed ${stamp}`);

  console.log("mobile-android-lifecycle: requesting graceful native exit");
  await nativeInvoke(page, "native_backend_debug_request_exit").catch(() => {});
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

import assert from "node:assert/strict";
import {
  openMobileSidebarView,
  readPendingAckCount,
  typeEditor,
} from "./mobile-webview-interaction.mjs";
import { commitSourceControlChange } from "./mobile-source-control-interaction.mjs";
import { createAndSelectAndroidDocument } from "./android-document-create-flow.mjs";

export async function dispatchWebViewText(page, value) {
  if (!/^[A-Za-z0-9 _-]+$/.test(value)) {
    throw new Error(`Android business input contains unsupported WebView text: ${value}`);
  }
  for (const character of value) {
    await page.send("Input.dispatchKeyEvent", {
      type: "char",
      text: character,
      unmodifiedText: character,
    });
  }
}

export async function clickVisible(page, selector) {
  const clicked = await page.call((target) => {
    const element = globalThis.__deveVisibleElement(target);
    if (!element) return false;
    element.click();
    return true;
  }, selector);
  if (!clicked) throw new Error(`visible click target not found: ${selector}`);
}

export async function fillVisible(page, selector, value) {
  const filled = await page.call((target, nextValue) => {
    const element = globalThis.__deveVisibleElement(target);
    if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) return false;
    const prototype = element instanceof HTMLTextAreaElement
      ? HTMLTextAreaElement.prototype
      : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(prototype, "value").set.call(element, nextValue);
    element.dispatchEvent(new InputEvent("input", {
      bubbles: true,
      inputType: "insertText",
      data: nextValue,
    }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  }, selector, value);
  if (!filled) throw new Error(`visible form field not found: ${selector}`);
}

export async function waitForWritableEditor(
  page,
  waitUntil,
  timeout = 30000,
  expectedDocId = null,
) {
  const readAdmission = (requiredDocId) => {
    const isVisible = (element) => {
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return style.display !== "none"
        && style.visibility !== "hidden"
        && rect.width > 0
        && rect.height > 0;
    };
    const visibleHosts = [...document.querySelectorAll("[data-deve-editor-host=true]")]
      .filter(isVisible);
    const candidates = requiredDocId === null
      ? visibleHosts
      : visibleHosts.filter((host) =>
        host.getAttribute("data-deve-editor-doc-id") === requiredDocId);
    if (candidates.length !== 1 || (requiredDocId !== null && visibleHosts.length !== 1)) {
      return { visible: false, writable: false };
    }
    const host = candidates[0];
    const codeHost = [...host.querySelectorAll("[data-deve-editor-codemirror-host=true]")]
      .find(isVisible);
    const content = codeHost
      ? [...codeHost.querySelectorAll(".cm-content")].find(isVisible)
      : null;
    const bootstrap = globalThis.__deveWebBridge?.get?.("__deveEditorBootstrap");
    return {
      visible: true,
      writable: host.getAttribute("data-deve-editor-readonly") === "false"
      && content?.getAttribute("contenteditable") === "true"
      && codeHost?.isConnected === true
      && bootstrap?.editorBridgeReady === true
      && bootstrap?.activeHost === codeHost,
    };
  };
  await waitUntil("visible Android editor", async () =>
    (await page.call(readAdmission, expectedDocId)).visible, timeout);
  await waitUntil("writable Android editor", async () =>
    (await page.call(readAdmission, expectedDocId)).writable, timeout);
}

export function readRemoteEntryState(expectedOrigin) {
  let requiredOrigin;
  try {
    requiredOrigin = new URL(expectedOrigin).origin;
  } catch {
    return { kind: "invalid-origin" };
  }
  if (location.origin !== requiredOrigin) return { kind: "unexpected-origin" };
  const status = document.querySelector("[data-deve-sync-status]")
    ?.getAttribute("data-deve-sync-status");
  if (status === "ready") return { kind: "ready" };
  const username = globalThis.__deveVisibleElement("#login-username");
  const password = globalThis.__deveVisibleElement("#login-password");
  const submit = globalThis.__deveVisibleElement('button[type="submit"]');
  return username && password && submit && !submit.disabled
    ? { kind: "login" }
    : null;
}

export function fillRemoteLoginCredentials(expectedOrigin, username, password) {
  let requiredOrigin;
  try {
    requiredOrigin = new URL(expectedOrigin).origin;
  } catch {
    return { kind: "invalid-origin" };
  }
  if (location.origin !== requiredOrigin) return { kind: "unexpected-origin" };
  const usernameInput = globalThis.__deveVisibleElement("#login-username");
  const passwordInput = globalThis.__deveVisibleElement("#login-password");
  if (!(usernameInput instanceof HTMLInputElement)
    || !(passwordInput instanceof HTMLInputElement)) {
    return { kind: "login-unavailable" };
  }
  for (const [element, value] of [[usernameInput, username], [passwordInput, password]]) {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(element, value);
    element.dispatchEvent(new InputEvent("input", {
      bubbles: true,
      inputType: "insertText",
      data: value,
    }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
  }
  return { kind: "credentials-filled" };
}

export function submitRemoteLogin(expectedOrigin) {
  let requiredOrigin;
  try {
    requiredOrigin = new URL(expectedOrigin).origin;
  } catch {
    return { kind: "invalid-origin" };
  }
  if (location.origin !== requiredOrigin) return { kind: "unexpected-origin" };
  const submit = globalThis.__deveVisibleElement('button[type="submit"]');
  if (!submit || submit.disabled) return { kind: "login-unavailable" };
  submit.click();
  return { kind: "submitted" };
}

export function readRemoteReadyState(expectedOrigin) {
  let requiredOrigin;
  try {
    requiredOrigin = new URL(expectedOrigin).origin;
  } catch {
    return { kind: "invalid-origin" };
  }
  if (location.origin !== requiredOrigin) return { kind: "unexpected-origin" };
  return document.querySelector("[data-deve-sync-status]")
    ?.getAttribute("data-deve-sync-status") === "ready"
    ? { kind: "ready" }
    : null;
}

export async function loginAndroidRemote(
  page,
  expectedOrigin,
  username,
  password,
  waitUntil,
) {
  const entry = await waitUntil("remote Android entry state", () =>
    page.call(readRemoteEntryState, expectedOrigin), 60000);
  if (entry.kind === "unexpected-origin" || entry.kind === "invalid-origin") {
    throw new Error(`remote Android entry rejected: ${entry.kind}`);
  }
  if (entry.kind === "login") {
    const filled = await page.call(
      fillRemoteLoginCredentials,
      expectedOrigin,
      username,
      password,
    );
    if (filled.kind !== "credentials-filled") {
      throw new Error(`remote Android credentials rejected: ${filled.kind}`);
    }
    const submitted = await page.call(submitRemoteLogin, expectedOrigin);
    if (submitted.kind !== "submitted") {
      throw new Error(`remote Android login submit rejected: ${submitted.kind}`);
    }
  }
  const ready = await waitUntil("remote Android ready", () =>
    page.call(readRemoteReadyState, expectedOrigin), 60000);
  if (ready.kind !== "ready") {
    throw new Error(`remote Android ready rejected: ${ready.kind}`);
  }
}

export async function createAndroidDocument(
  page,
  path,
  content,
  { waitUntil, inputEditorText },
) {
  const selected = await createAndSelectAndroidDocument(page, path, {
    waitUntil,
    click: clickVisible,
    fill: fillVisible,
  });
  await waitForWritableEditor(page, waitUntil, 30000, selected.docId);
  await waitUntil("editor bridge", () => page.call(() =>
    typeof window.getEditorContent === "function" && typeof window.getEditorContent() === "string"));
  const observedContent = await typeEditor(
    page,
    content,
    waitUntil,
    inputEditorText,
    selected.docId,
  );
  await waitUntil("Android edit ack", async () => (await readPendingAckCount(page)) === 0);
  return observedContent;
}

export function commitAndroidChange(page, message, { waitUntil, delay }) {
  return commitSourceControlChange(page, message, {
    click: clickVisible,
    waitUntil,
    delay,
    inputText: async (value) => dispatchWebViewText(page, value),
  });
}

async function readRepoScope(page) {
  const raw = await page.call(() => {
    const status = document.querySelector("[data-deve-sync-status]");
    return {
      status: status?.getAttribute("data-deve-sync-status") ?? null,
      repoIdRaw: status?.getAttribute("data-deve-repo-id") ?? null,
      scopeNonceRaw: status?.getAttribute("data-deve-scope-nonce") ?? null,
    };
  });
  const scopeNonce = typeof raw.scopeNonceRaw === "string"
    && /^(0|[1-9][0-9]*)$/.test(raw.scopeNonceRaw)
    ? Number(raw.scopeNonceRaw)
    : null;
  return {
    ...raw,
    repoId: raw.repoIdRaw,
    scopeNonce: Number.isSafeInteger(scopeNonce) ? scopeNonce : null,
  };
}

export async function waitForStableAndroidRepoScope(
  page,
  waitUntil,
  {
    expectedRepoId,
    minimumScopeNonce,
    quietMs = 1000,
    now = Date.now,
  },
) {
  let candidate = null;
  let stableSince = null;
  return waitUntil("stable Android repo writer scope", async () => {
    const current = await readRepoScope(page);
    const valid = current.status === "ready"
      && current.repoId === expectedRepoId
      && Number.isInteger(current.scopeNonce)
      && current.scopeNonce >= minimumScopeNonce;
    if (!valid) {
      candidate = null;
      stableSince = null;
      return null;
    }
    const identity = `${current.repoId}\u0000${current.scopeNonce}`;
    const observedAt = now();
    if (candidate !== identity) {
      candidate = identity;
      stableSince = observedAt;
      return quietMs === 0 ? current : null;
    }
    return observedAt - stableSince >= quietMs ? current : null;
  }, 30000);
}

async function openRepoSwitcher(page, waitUntil) {
  const mobile = await page.call(() =>
    Boolean(globalThis.__deveVisibleElement('[data-deve-layout-mode="mobile"]')));
  if (mobile) {
    await openMobileSidebarView(page, "explorer", {
      click: clickVisible,
      waitUntil,
    });
  }
  await clickVisible(page, "[data-deve-repo-switcher-trigger]");
  await waitUntil("Android repo switcher", () => page.call(() =>
    Boolean(globalThis.__deveVisibleElement("[data-deve-repo-switcher-create]"))));
}

export async function createFirstAndroidRepoFromBootstrapUnbound(
  page,
  name,
  { waitUntil, stableQuietMs = 1000, stableNow = Date.now },
) {
  const initial = await waitUntil("initial zero-repo BootstrapUnbound", async () => {
    const current = await readRepoScope(page);
    return current.status === "handshaking-repo"
      && current.repoIdRaw === ""
      && current.scopeNonceRaw === "0"
      && current.scopeNonce === 0
      ? current
      : null;
  });
  assert.notEqual(
    initial.status,
    "ready",
    "zero-repo startup must not claim repo writer readiness",
  );

  await openRepoSwitcher(page, waitUntil);
  const existing = await page.call(() =>
    document.querySelectorAll("[data-deve-repo-switcher-item]").length);
  assert.equal(existing, 0, "fresh LocalBackend must not auto-create a default repo");
  await clickVisible(page, "[data-deve-repo-switcher-create]");
  await waitUntil("Android first repo create input", () => page.call(() =>
    Boolean(globalThis.__deveVisibleElement("[data-deve-repo-switcher-create-input]"))));
  await fillVisible(page, "[data-deve-repo-switcher-create-input]", name);
  const submitted = await page.call(() => {
    const input = globalThis.__deveVisibleElement("[data-deve-repo-switcher-create-input]");
    const submit = input?.closest("form")?.querySelector('button[type="submit"]');
    if (!(submit instanceof HTMLButtonElement) || submit.disabled) return false;
    submit.click();
    return true;
  });
  assert.equal(submitted, true, "first Create must use the visible repo switcher form");

  const firstReady = await waitUntil("first Android repo writer readiness", async () => {
    const current = await readRepoScope(page);
    return current.status === "ready"
      && typeof current.repoIdRaw === "string"
      && current.repoIdRaw !== ""
      && current.scopeNonceRaw !== null
      && Number.isInteger(current.scopeNonce)
      && current.scopeNonce > initial.scopeNonce
      ? current
      : null;
  }, 60000);
  assert.ok(firstReady.repoId, "first Create must bind the backend-projected repo scope");
  assert.ok(
    firstReady.scopeNonce > initial.scopeNonce,
    "first Create must advance the backend scope nonce",
  );

  await openRepoSwitcher(page, waitUntil);
  const aliases = await page.call((expected) =>
    [...document.querySelectorAll("[data-deve-repo-switcher-item]")]
      .filter((item) => item.getAttribute("data-deve-repo-switcher-item-name") === expected)
      .length, name);
  assert.equal(aliases, 1, "first Create must publish the backend-owned display alias");
  await clickVisible(page, "[data-deve-repo-switcher-backdrop]");
  const created = await waitForStableAndroidRepoScope(page, waitUntil, {
    expectedRepoId: firstReady.repoId,
    minimumScopeNonce: firstReady.scopeNonce,
    quietMs: stableQuietMs,
    now: stableNow,
  });
  return {
    initial,
    created,
    name,
    defaultRepoAbsent: existing === 0,
    aliasCount: aliases,
  };
}

export async function exerciseAndroidLastRepoRemoval(page, { waitUntil }) {
  const before = await readRepoScope(page);
  assert.equal(before.status, "ready");
  assert.ok(before.repoId, "Android repo removal requires a selected repo");
  assert.ok(Number.isInteger(before.scopeNonce) && before.scopeNonce > 0);

  await openRepoSwitcher(page, waitUntil);
  const row = await page.call(() => {
    const items = [...document.querySelectorAll("[data-deve-repo-switcher-item]")];
    if (items.length !== 1) return { count: items.length };
    const item = items[0];
    const actions = item.parentElement?.querySelector("[data-deve-repo-switcher-actions]");
    if (!(actions instanceof HTMLElement)) return { count: items.length, actions: false };
    actions.click();
    return {
      count: items.length,
      actions: true,
      alias: item.getAttribute("data-deve-repo-switcher-item-name") ?? "",
    };
  });
  assert.equal(row.count, 1, "Android repo removal requires exactly one local repo");
  assert.equal(row.actions, true, "Android repo row must expose actions");
  assert.ok(row.alias, "Android last repo must expose a backend-projected alias");
  await clickVisible(page, "[data-deve-repo-switcher-remove]");
  await waitUntil("Android repo removal preview", () => page.call(() => {
    const dialog = globalThis.__deveVisibleElement('[data-deve-repo-removal-dialog="visible"]');
    const preserved = dialog?.querySelector("#repo-removal-preserved-heading");
    const confirm = dialog?.querySelector('[data-deve-repo-removal-confirm="true"]');
    return dialog && preserved && confirm instanceof HTMLButtonElement && !confirm.disabled;
  }));
  await clickVisible(page, '[data-deve-repo-removal-confirm="true"]');
  const noScope = await waitUntil("Android last repo NoScope finalization", async () => {
    const current = await readRepoScope(page);
    return current.repoId === "" ? current : null;
  });
  assert.ok(
    Number.isInteger(noScope.scopeNonce) && noScope.scopeNonce > before.scopeNonce,
    "Android last repo removal must advance the backend scope nonce",
  );
  await openRepoSwitcher(page, waitUntil);
  const remaining = await page.call(() =>
    document.querySelectorAll("[data-deve-repo-switcher-item]").length);
  assert.equal(remaining, 0, "Android NoScope must expose no local repo rows");
  await clickVisible(page, "[data-deve-repo-switcher-backdrop]");
  return {
    alias: row.alias,
    removedRepoId: before.repoId,
    scopeNonceBeforeRemoval: before.scopeNonce,
    scopeNonceAfterRemoval: noScope.scopeNonce,
    noScope: true,
  };
}

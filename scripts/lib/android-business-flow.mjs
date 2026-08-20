import assert from "node:assert/strict";
import {
  closeMobileSidebar,
  openMobileSidebarView,
  readPendingAckCount,
  typeEditor,
} from "./mobile-webview-interaction.mjs";
import { commitSourceControlChange } from "./mobile-source-control-interaction.mjs";
import { createAndSelectAndroidDocument } from "./android-document-create-flow.mjs";

export {
  fillRemoteLoginCredentials,
  loginAndroidRemote,
  readRemoteEntryState,
  readRemoteReadyState,
  submitRemoteLogin,
} from "./android-remote-auth-flow.mjs";

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

async function clickVisibleInExactRepoScope(page, selector, expected, label) {
  const outcome = await page.call((target, identity) => {
    const status = document.querySelector("[data-deve-sync-status]");
    if (status?.getAttribute("data-deve-sync-status") !== "ready") {
      return "status-not-ready";
    }
    if (status.getAttribute("data-deve-repo-id") !== identity.repoId) {
      return "repo-identity-mismatch";
    }
    if (status.getAttribute("data-deve-scope-nonce") !== String(identity.scopeNonce)) {
      return "scope-nonce-mismatch";
    }
    const element = globalThis.__deveVisibleElement(target);
    if (!element) return "target-unavailable";
    element.click();
    return "clicked";
  }, selector, expected);
  if (outcome !== "clicked") {
    throw new Error(`${label} rejected: ${outcome}`);
  }
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

export async function createAndroidDocument(
  page,
  path,
  content,
  { waitUntil, inputEditorText },
) {
  const expectedWriterScope = await waitForCurrentStableAndroidRepoScope(page, waitUntil);
  const selected = await createAndSelectAndroidDocument(page, path, {
    waitUntil,
    click: clickVisible,
    fill: fillVisible,
    expectedWriterScope,
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

export async function waitForCurrentStableAndroidRepoScope(
  page,
  waitUntil,
  { quietMs = 1000, now = () => globalThis.performance.now() } = {},
) {
  const current = await waitUntil("current Android repo writer scope", async () => {
    const observed = await readRepoScope(page);
    return observed.status === "ready"
      && typeof observed.repoId === "string"
      && observed.repoId.length > 0
      && Number.isInteger(observed.scopeNonce)
      && observed.scopeNonce > 0
      ? observed
      : null;
  }, 30000);
  return waitForStableAndroidRepoScope(page, waitUntil, {
    expectedRepoId: current.repoId,
    minimumScopeNonce: current.scopeNonce,
    quietMs,
    now,
  });
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
    now = () => globalThis.performance.now(),
  },
) {
  assert.ok(Number.isFinite(quietMs) && quietMs >= 0, "stable scope quiet window is invalid");
  assert.equal(typeof now, "function", "stable scope clock is invalid");
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
    assert.ok(Number.isFinite(observedAt) && observedAt >= 0, "stable scope clock failed");
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
  { waitUntil, stableQuietMs = 1000, stableNow = () => globalThis.performance.now() },
) {
  let lastInitial = null;
  const initial = await waitUntil("initial zero-repo BootstrapUnbound", async () => {
    lastInitial = await readRepoScope(page);
    return lastInitial.status === "handshaking-repo"
      && lastInitial.repoIdRaw === ""
      && lastInitial.scopeNonceRaw === "0"
      && lastInitial.scopeNonce === 0
      ? lastInitial
      : null;
  }).catch((error) => {
    const diagnostic = {
      status: lastInitial?.status === "ready" || lastInitial?.status === "handshaking-repo"
        ? lastInitial.status
        : "other",
      repoIdPresent: typeof lastInitial?.repoIdRaw === "string"
        && lastInitial.repoIdRaw.length > 0,
      scopeNonce: Number.isSafeInteger(lastInitial?.scopeNonce) ? lastInitial.scopeNonce : null,
    };
    throw new Error(`${error.message}; last_scope=${JSON.stringify(diagnostic)}`);
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
  await closeMobileSidebar(page, { click: clickVisible, waitUntil });
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

export async function exerciseAndroidLastRepoRemoval(
  page,
  {
    waitUntil,
    expectedRepoId,
    minimumScopeNonce,
    stableQuietMs = 1000,
    stableNow = () => globalThis.performance.now(),
  },
) {
  assert.ok(expectedRepoId, "Android repo removal requires an expected repo identity");
  assert.ok(
    Number.isInteger(minimumScopeNonce) && minimumScopeNonce > 0,
    "Android repo removal requires a positive minimum scope nonce",
  );
  const before = await waitForStableAndroidRepoScope(page, waitUntil, {
    expectedRepoId,
    minimumScopeNonce,
    quietMs: stableQuietMs,
    now: stableNow,
  });

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
  await clickVisibleInExactRepoScope(
    page,
    "[data-deve-repo-switcher-remove]",
    before,
    "Android repo removal preview intent",
  );
  await waitUntil("Android repo removal preview", () => page.call(() => {
    const dialog = globalThis.__deveVisibleElement('[data-deve-repo-removal-dialog="visible"]');
    const preserved = dialog?.querySelector("#repo-removal-preserved-heading");
    const confirm = dialog?.querySelector('[data-deve-repo-removal-confirm="true"]');
    return dialog && preserved && confirm instanceof HTMLButtonElement && !confirm.disabled;
  }));
  await clickVisibleInExactRepoScope(
    page,
    '[data-deve-repo-removal-confirm="true"]',
    before,
    "Android repo removal execute intent",
  );
  const noScope = await waitUntil("Android last repo NoScope finalization", async () => {
    const current = await readRepoScope(page);
    return current.status === "handshaking-repo"
      && current.repoId === ""
      && Number.isInteger(current.scopeNonce)
      && current.scopeNonce > before.scopeNonce
      ? current
      : null;
  });
  assert.equal(noScope.status, "handshaking-repo");
  assert.ok(
    noScope.scopeNonce > before.scopeNonce,
    "Android last repo removal must advance the backend scope nonce",
  );
  await openRepoSwitcher(page, waitUntil);
  const remaining = await page.call(() =>
    document.querySelectorAll("[data-deve-repo-switcher-item]").length);
  assert.equal(remaining, 0, "Android NoScope must expose no local repo rows");
  await clickVisible(page, "[data-deve-repo-switcher-backdrop]");
  await closeMobileSidebar(page, { click: clickVisible, waitUntil });
  return {
    alias: row.alias,
    removedRepoId: before.repoId,
    scopeNonceBeforeRemoval: before.scopeNonce,
    scopeNonceAfterRemoval: noScope.scopeNonce,
    noScope: true,
  };
}

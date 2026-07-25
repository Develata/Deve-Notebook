import assert from "node:assert/strict";
import { evaluateWritableProbeExpectation } from "./android-target-capability.mjs";
import { probeWebCryptoEd25519 } from "./webcrypto-capability.mjs";

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

export async function verifyAndroidIdentityCapability(
  page,
  { expectWritable, withDeadline, waitUntil },
) {
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

export async function proveAndroidReadonlyMutationRejected(page) {
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
  await new Promise((resolve) => setTimeout(resolve, 250));
  const after = await page.call(() => ({
    bridgeContent: window.getEditorContent?.() ?? null,
    domContent: globalThis.__deveVisibleElement(".cm-content")?.textContent ?? null,
    pending: document.querySelector("[data-deve-mobile-pending-ack-count]")
      ?.getAttribute("data-deve-mobile-pending-ack-count") ?? null,
  }));
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
  assert.equal(String(after.pending ?? "0"), String(before.pending));
  return { editorInputRejected: before.hasEditor, emptyRepoCreateRejected: !before.hasEditor };
}

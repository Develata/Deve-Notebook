import {
  openMobileSidebarView,
  readPendingAckCount,
  typeEditor,
} from "./mobile-webview-interaction.mjs";
import { commitSourceControlChange } from "./mobile-source-control-interaction.mjs";

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

export async function waitForWritableEditor(page, waitUntil, timeout = 30000) {
  await waitUntil("visible Android editor", () => page.call(() =>
    Boolean(globalThis.__deveVisibleElement("[data-deve-editor-host=true]"))), timeout);
  await waitUntil("writable Android editor", () => page.call(() => {
    const host = globalThis.__deveVisibleElement("[data-deve-editor-host=true]");
    const codeHost = globalThis.__deveVisibleElement("[data-deve-editor-codemirror-host=true]");
    const content = globalThis.__deveVisibleElement(".cm-content");
    const bootstrap = globalThis.__deveWebBridge?.get?.("__deveEditorBootstrap");
    return host?.getAttribute("data-deve-editor-readonly") === "false"
      && content?.getAttribute("contenteditable") === "true"
      && codeHost?.isConnected === true
      && bootstrap?.editorBridgeReady === true
      && bootstrap?.activeHost === codeHost;
  }), timeout);
}

export async function loginAndroidRemote(page, username, password, waitUntil) {
  const loginVisible = await page.call(() =>
    Boolean(globalThis.__deveVisibleElement("#login-username")));
  if (loginVisible) {
    await fillVisible(page, "#login-username", username);
    await fillVisible(page, "#login-password", password);
    await clickVisible(page, 'button[type="submit"]');
  }
  await waitUntil("remote Android ready", () => page.call(() =>
    document.querySelector("[data-deve-sync-status]")
      ?.getAttribute("data-deve-sync-status") === "ready"), 60000);
}

export async function createAndroidDocument(
  page,
  path,
  content,
  { waitUntil, inputEditorText },
) {
  const mobile = await page.call(() =>
    Boolean(globalThis.__deveVisibleElement('[data-deve-layout-mode="mobile"]')));
  if (mobile) {
    await openMobileSidebarView(page, "explorer", {
      click: clickVisible,
      waitUntil,
    });
  }
  await clickVisible(page, "[data-deve-new-doc-button=true]");
  await waitUntil("new document input", () => page.call(() =>
    Boolean(globalThis.__deveVisibleElement("[data-deve-search-input=true]"))));
  await fillVisible(page, "[data-deve-search-input=true]", `+${path}`);
  await waitUntil("create document action", () => page.call(() =>
    Boolean(globalThis.__deveVisibleElement('[data-deve-search-result-action="create-doc"]'))));
  await clickVisible(page, '[data-deve-search-result-action="create-doc"]');
  await waitForWritableEditor(page, waitUntil);
  await waitUntil("editor bridge", () => page.call(() =>
    typeof window.getEditorContent === "function" && typeof window.getEditorContent() === "string"));
  const observedContent = await typeEditor(page, content, waitUntil, inputEditorText);
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

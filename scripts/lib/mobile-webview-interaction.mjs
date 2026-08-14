import { clickWebViewPoint } from "./android-webview-pointer.mjs";
import {
  readEditorMountObservation,
  sameEditorLoadSession,
  sameEditorSelectionIdentity,
} from "./mobile-editor-session-observation.mjs";

export {
  readEditorMountObservation,
  sameEditorLoadSession,
  sameEditorSelectionIdentity,
};

export function editorFocusMatchesMode(contentEditable, writable, activeEditor = true) {
  if (!activeEditor) return false;
  return writable ? contentEditable === "true" : contentEditable !== "true";
}

export function parsePendingAckCount(value) {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) {
    throw new Error(`android_pending_ack_marker_invalid: ${JSON.stringify(value)}`);
  }
  return Number(value);
}

export async function readPendingAckCount(page) {
  const value = await page.call(() =>
    document.querySelector("[data-deve-mobile-pending-ack-count]")
      ?.getAttribute("data-deve-mobile-pending-ack-count")
    ?? document.querySelector("[data-deve-pending-ack-count]")
      ?.getAttribute("data-deve-pending-ack-count")
    ?? null);
  return parsePendingAckCount(value);
}

export async function readSourceControlCommitState(page) {
  return page.call(() => {
    const field = globalThis.__deveVisibleElement('textarea[name="commit-message"]');
    const panel = field?.closest("div.border-t");
    const button = panel?.querySelector("button:has(.codicon-check)");
    return {
      message: field?.value ?? null,
      fieldDisabled: field?.disabled ?? null,
      buttonDisabled: button?.disabled ?? null,
      buttonTitle: button?.title ?? null,
      confirmedCount: panel?.parentElement
        ?.querySelectorAll(
          '[data-deve-sc-section-body="confirmed-ledger"] '
            + '[data-deve-mobile-touch-target="source-control-change-row"]',
        ).length
        ?? null,
    };
  });
}

export function sourceControlCommitReady(state, expectedMessage) {
  return state?.confirmedCount > 0
    && state.message === expectedMessage
    && state.fieldDisabled === false
    && state.buttonDisabled === false;
}

export function sourceControlCommitAcknowledged(state) {
  return state?.confirmedCount === 0 && state.message === "";
}

export async function focusEditor(
  page,
  { writable = true, requireFocused = true, expectedDocId = null } = {},
) {
  const point = await page.call((requiredDocId) => {
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
      return null;
    }
    const codeHost = [...candidates[0].querySelectorAll(
      "[data-deve-editor-codemirror-host=true]",
    )].find(isVisible);
    const editor = codeHost
      ? [...codeHost.querySelectorAll(".cm-content")].find(isVisible)
      : null;
    if (!editor) return null;
    if (requiredDocId !== null) {
      const bootstrap = globalThis.__deveWebBridge?.get?.("__deveEditorBootstrap");
      if (bootstrap?.editorBridgeReady !== true || bootstrap?.activeHost !== codeHost) return null;
    }
    editor.focus();
    const selection = getSelection();
    const range = document.createRange();
    range.selectNodeContents(editor);
    range.collapse(false);
    selection?.removeAllRanges();
    selection?.addRange(range);
    const rect = editor.getBoundingClientRect();
    const viewport = window.visualViewport;
    const viewportLeft = viewport?.offsetLeft ?? 0;
    const viewportTop = viewport?.offsetTop ?? 0;
    const viewportRight = viewportLeft + (viewport?.width ?? window.innerWidth);
    const viewportBottom = viewportTop + (viewport?.height ?? window.innerHeight);
    const left = Math.max(rect.left, viewportLeft);
    const right = Math.min(rect.right, viewportRight);
    const top = Math.max(rect.top, viewportTop);
    const bottom = Math.min(rect.bottom, viewportBottom);
    if (right <= left || bottom <= top) return null;
    return {
      x: left + Math.min(24, (right - left) / 2),
      y: top + Math.min(24, (bottom - top) / 2),
      devicePixelRatio: window.devicePixelRatio || 1,
      rect: { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
    };
  }, expectedDocId);
  if (!point) {
    throw new Error(
      expectedDocId === null
        ? "visible CodeMirror editor not found"
        : "exact document CodeMirror editor not admitted",
    );
  }
  const focusState = await page.call((requiredDocId) => {
    const active = document.activeElement;
    const activeHost = active?.closest?.("[data-deve-editor-host=true]") ?? null;
    const activeCodeHost = active?.closest?.("[data-deve-editor-codemirror-host=true]") ?? null;
    const bootstrap = globalThis.__deveWebBridge?.get?.("__deveEditorBootstrap");
    const visibleHosts = [...document.querySelectorAll("[data-deve-editor-host=true]")]
      .filter((host) => {
        const style = getComputedStyle(host);
        const rect = host.getBoundingClientRect();
        return style.display !== "none"
          && style.visibility !== "hidden"
          && rect.width > 0
          && rect.height > 0;
      });
    return {
      tag: active?.tagName ?? null,
      className: active?.className ?? null,
      contentEditable: active?.getAttribute("contenteditable") ?? null,
      activeEditor: active?.classList?.contains("cm-content") ?? false,
      identityMatched: requiredDocId === null || (
        visibleHosts.length === 1
        && activeHost === visibleHosts[0]
        && activeHost?.getAttribute("data-deve-editor-doc-id") === requiredDocId
        && bootstrap?.editorBridgeReady === true
        && bootstrap?.activeHost === activeCodeHost
      ),
      visualViewportHeight: window.visualViewport?.height ?? null,
    };
  }, expectedDocId);
  console.log(`mobile-android-webview: editor focus ${JSON.stringify({ point, focusState })}`);
  if (requireFocused && (
    !editorFocusMatchesMode(
      focusState.contentEditable,
      writable,
      focusState.activeEditor,
    ) || (expectedDocId !== null && !focusState.identityMatched)
  )) {
    throw new Error(
      `android_webview_editor_focus_mode_mismatch: ${JSON.stringify({ writable, focusState })}`,
    );
  }
  return point;
}

export async function focusAndroidEditorInputConnection(
  page,
  {
    tap = clickWebViewPoint,
    delay,
    waitForWritableEditor,
    observeEditor = readEditorMountObservation,
    expectedDocId = null,
  },
) {
  let lastFocusError;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    await waitForWritableEditor(page, expectedDocId);
    const before = await observeEditor(page, expectedDocId);
    if (!before?.point || !before.bridgeReady || !before.activeHostMatchesVisible) {
      lastFocusError = new Error(
        `android_webview_editor_input_session_unready: ${JSON.stringify({
          hostId: before?.hostId ?? null,
          openRequestId: before?.openRequestId ?? null,
        })}`,
      );
      await delay(250);
      continue;
    }
    await tap(page, before.point);
    await delay(250);
    await waitForWritableEditor(page, expectedDocId);
    const after = await observeEditor(page, expectedDocId);
    if (!sameEditorLoadSession(before, after)) {
      lastFocusError = new Error(
        `android_webview_editor_input_session_changed: ${JSON.stringify({
          before: { hostId: before.hostId, openRequestId: before.openRequestId },
          after: {
            hostId: after?.hostId ?? null,
            openRequestId: after?.openRequestId ?? null,
          },
        })}`,
      );
      await delay(250);
      continue;
    }
    try {
      await focusEditor(page, { expectedDocId });
      const focused = await observeEditor(page, expectedDocId);
      if (!sameEditorLoadSession(after, focused)
        || !focused?.activeEditor
        || !focused.bridgeReady
        || !focused.activeHostMatchesVisible) {
        throw new Error("exact editor session changed while establishing focus");
      }
      return focused;
    } catch (error) {
      lastFocusError = error;
      await delay(250);
    }
  }
  throw lastFocusError;
}

export async function typeAndroidEditorText(
  page,
  content,
  {
    tap = clickWebViewPoint,
    delay,
    waitForWritableEditor,
    inputText,
    observeEditor = readEditorMountObservation,
    expectedDocId = null,
  },
) {
  let settled = false;
  for (let attempt = 0; attempt < 2 && !settled; attempt += 1) {
    const focused = await focusAndroidEditorInputConnection(page, {
      tap,
      delay,
      waitForWritableEditor,
      observeEditor,
      expectedDocId,
    });
    // Android WebView can report DOM focus before its native input connection
    // has settled after a real pointer gesture.
    await delay(300);
    await waitForWritableEditor(page, expectedDocId);
    const afterSettle = await observeEditor(page, expectedDocId);
    settled = sameEditorLoadSession(focused, afterSettle)
      && afterSettle?.activeEditor
      && afterSettle.bridgeReady
      && afterSettle.activeHostMatchesVisible;
  }
  if (!settled) throw new Error("exact editor input session changed before text insertion");
  await inputText(content);
}

export async function typeEditor(
  page,
  content,
  waitUntil,
  inputText,
  expectedDocId = null,
) {
  if (typeof inputText === "function") {
    // The native driver owns bounded focus/remount recovery. A redundant
    // one-shot DOM focus here would fail before that recovery can run.
    await inputText(content, null, page, expectedDocId);
  } else {
    await focusEditor(page, { expectedDocId });
    await page.send("Input.insertText", { text: content });
  }
  return waitUntil("editor input", () => page.call(
    (expected, requiredDocId) => {
      if (requiredDocId !== null) {
        const visibleHosts = [...document.querySelectorAll("[data-deve-editor-host=true]")]
          .filter((host) => {
            const style = getComputedStyle(host);
            const rect = host.getBoundingClientRect();
            return style.display !== "none"
              && style.visibility !== "hidden"
              && rect.width > 0
              && rect.height > 0;
          });
        const host = visibleHosts.length === 1
          && visibleHosts[0].getAttribute("data-deve-editor-doc-id") === requiredDocId
          ? visibleHosts[0]
          : null;
        const codeHost = host?.querySelector("[data-deve-editor-codemirror-host=true]");
        const bootstrap = globalThis.__deveWebBridge?.get?.("__deveEditorBootstrap");
        if (!host || bootstrap?.editorBridgeReady !== true || bootstrap?.activeHost !== codeHost) {
          return null;
        }
      }
      const observed = window.getEditorContent?.();
      return observed?.includes(expected) ? observed : null;
    },
    content,
    expectedDocId,
  ), 5000).catch(async (error) => {
    const observed = await page.call((requiredDocId) => {
      const active = document.activeElement;
      const activeHost = active?.closest?.("[data-deve-editor-host=true]") ?? null;
      const bootstrap = globalThis.__deveWebBridge?.get?.("__deveEditorBootstrap");
      return {
        bridge: window.getEditorContent?.() ?? null,
        dom: activeHost?.querySelector(".cm-content")?.textContent ?? null,
        activeClass: active?.className ?? null,
        activeEditable: active?.getAttribute?.("contenteditable") ?? null,
        activeHostConnected: bootstrap?.activeHost?.isConnected ?? null,
        activeDocIdMatched: requiredDocId === null
          || activeHost?.getAttribute("data-deve-editor-doc-id") === requiredDocId,
        activeHostMatchesVisible: bootstrap?.activeHost
          === active?.closest?.("[data-deve-editor-codemirror-host=true]"),
      };
    }, expectedDocId).catch(() => null);
    throw new Error(
      `android_webview_input_injection_unavailable: ${error.message}; observed=${JSON.stringify(observed)}`,
    );
  });
}

export async function typeAndroidTextField(page, selector, value, { tap, delay, inputText }) {
  let focused = false;
  for (let attempt = 0; attempt < 5 && !focused; attempt += 1) {
    const point = await page.call((target) => {
      const element = globalThis.__deveVisibleElement(target);
      if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) return null;
      const rect = element.getBoundingClientRect();
      return {
        x: rect.left + Math.min(24, rect.width / 2),
        y: rect.top + Math.min(24, rect.height / 2),
        devicePixelRatio: window.devicePixelRatio || 1,
      };
    }, selector);
    if (!point) throw new Error(`visible text field could not receive focus: ${selector}`);
    await tap(point);
    await delay(300);
    focused = await page.call((target) => {
      const element = globalThis.__deveVisibleElement(target);
      if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) return false;
      element.click();
      element.focus();
      element.select();
      return element.isConnected && document.activeElement === element;
    }, selector);
  }
  if (!focused) {
    const diagnostics = await page.call((target) => ({
      activeTag: document.activeElement?.tagName ?? null,
      activeClass: document.activeElement?.className ?? null,
      fieldConnected: globalThis.__deveVisibleElement(target)?.isConnected ?? null,
      fieldDisabled: globalThis.__deveVisibleElement(target)?.disabled ?? null,
      drawerOpen: document.querySelector('[data-deve-mobile-drawer="left"]')
        ?.getAttribute("data-deve-mobile-drawer-open") ?? null,
    }), selector);
    throw new Error(`Android text field focus mismatch: ${selector}; ${JSON.stringify(diagnostics)}`);
  }
  if (typeof inputText === "function") {
    await inputText(value);
  } else {
    await page.send("Input.insertText", { text: value });
  }
}

export async function openMobileSidebarView(page, view, { click, waitUntil }) {
  const drawer = '[data-deve-mobile-drawer="left"]';
  const isOpen = await page.call((selector) =>
    document.querySelector(selector)?.getAttribute("data-deve-mobile-drawer-open") === "true", drawer);
  if (!isOpen) {
    await click(page, '[data-deve-mobile-header-action="open_left_drawer"]');
    await waitUntil("mobile left drawer", () => page.call((selector) =>
      document.querySelector(selector)?.getAttribute("data-deve-mobile-drawer-open") === "true", drawer), 10000);
  }
  const tab = `[data-deve-mobile-sidebar-tab="${view}"]`;
  await click(page, tab);
  await waitUntil(`mobile ${view} sidebar`, () => page.call((selector) =>
    document.querySelector(selector)?.getAttribute("data-deve-mobile-sidebar-tab-active") === "true", tab), 10000);
}

export async function closeMobileSidebar(page, { click, waitUntil }) {
  const drawer = '[data-deve-mobile-drawer="left"]';
  const isOpen = await page.call((selector) =>
    document.querySelector(selector)?.getAttribute("data-deve-mobile-drawer-open") === "true", drawer);
  if (!isOpen) return;
  await click(page, `${drawer} [data-deve-mobile-touch-target="drawer_close_buttons"]`);
  await waitUntil("closed mobile left drawer", () => page.call((selector) =>
    document.querySelector(selector)?.getAttribute("data-deve-mobile-drawer-open") === "false", drawer), 10000);
}

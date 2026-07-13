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

export async function clickWebViewPoint(page, point) {
  await page.send("Input.dispatchMouseEvent", {
    type: "mousePressed",
    x: point.x,
    y: point.y,
    button: "left",
    clickCount: 1,
  });
  await page.send("Input.dispatchMouseEvent", {
    type: "mouseReleased",
    x: point.x,
    y: point.y,
    button: "left",
    clickCount: 1,
  });
}

export async function readEditorMountObservation(page) {
  return page.call(() => {
    const host = globalThis.__deveVisibleElement("[data-deve-editor-host=true]");
    const codeHost = globalThis.__deveVisibleElement(
      "[data-deve-editor-codemirror-host=true]",
    );
    const bootstrap = globalThis.__deveWebBridge?.get?.("__deveEditorBootstrap");
    const registry = globalThis.__deveSmokeEditorHostRegistry ??= {
      ids: new WeakMap(),
      nextId: 1,
    };
    let hostId = null;
    if (host) {
      hostId = registry.ids.get(host) ?? registry.nextId++;
      registry.ids.set(host, hostId);
    }
    return {
      hostId,
      openRequestId: host?.getAttribute("data-deve-editor-open-request-id") ?? null,
      viewportWidth: window.innerWidth,
      innerHeight: window.innerHeight,
      visualViewportHeight: window.visualViewport?.height ?? window.innerHeight,
      activeEditor: document.activeElement?.classList?.contains("cm-content") ?? false,
      bridgeReady: bootstrap?.editorBridgeReady === true,
      activeHostMatchesVisible: bootstrap?.activeHost === codeHost,
    };
  });
}

export function sameEditorLoadSession(before, after) {
  return before?.hostId != null
    && before.hostId === after?.hostId
    && before.openRequestId != null
    && before.openRequestId === after?.openRequestId;
}

export async function proveSameBreakpointKeyboardResize(
  page,
  { waitUntil, activateKeyboard, maxViewportWidth = 768, minHeightDelta = 80 },
) {
  const baseline = await waitUntil("Android keyboard hidden baseline", async () => {
    const observation = await readEditorMountObservation(page);
    return observation.innerHeight - observation.visualViewportHeight < minHeightDelta
      ? observation
      : null;
  }, 10000);
  const point = await focusEditor(page, { requireFocused: false });
  await activateKeyboard(point, page);
  const resized = await waitUntil("same-breakpoint Android keyboard resize", async () => {
    const observation = await readEditorMountObservation(page);
    return observation.activeEditor
      && observation.bridgeReady
      && observation.activeHostMatchesVisible
      && observation.visualViewportHeight < baseline.visualViewportHeight - minHeightDelta
      ? observation
      : null;
  }, 10000);
  if (!sameEditorLoadSession(baseline, resized)) {
    throw new Error("same-breakpoint keyboard resize replaced editor host or OpenDoc request");
  }
  if (baseline.viewportWidth > maxViewportWidth || resized.viewportWidth > maxViewportWidth) {
    throw new Error("keyboard resize crossed the expected mobile breakpoint");
  }
  return { baseline, resized };
}

export async function focusEditor(page, { writable = true, requireFocused = true } = {}) {
  const point = await page.call(() => {
    const editor = globalThis.__deveVisibleElement(".cm-content");
    if (!editor) return null;
    editor.focus();
    const selection = getSelection();
    const range = document.createRange();
    range.selectNodeContents(editor);
    range.collapse(false);
    selection?.removeAllRanges();
    selection?.addRange(range);
    const rect = editor.getBoundingClientRect();
    return {
      x: rect.left + Math.min(24, rect.width / 2),
      y: rect.top + Math.min(24, rect.height / 2),
      devicePixelRatio: window.devicePixelRatio || 1,
      rect: { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
    };
  });
  if (!point) throw new Error("visible CodeMirror editor not found");
  const focusState = await page.call(() => ({
    tag: document.activeElement?.tagName ?? null,
    className: document.activeElement?.className ?? null,
    contentEditable: document.activeElement?.getAttribute("contenteditable") ?? null,
    activeEditor: document.activeElement?.classList?.contains("cm-content") ?? false,
    visualViewportHeight: window.visualViewport?.height ?? null,
  }));
  console.log(`mobile-android-lifecycle: editor focus ${JSON.stringify({ point, focusState })}`);
  if (requireFocused && !editorFocusMatchesMode(
    focusState.contentEditable,
    writable,
    focusState.activeEditor,
  )) {
    throw new Error(
      `android_webview_editor_focus_mode_mismatch: ${JSON.stringify({ writable, focusState })}`,
    );
  }
  return point;
}

export async function typeEditor(page, content, waitUntil, inputText) {
  const point = await focusEditor(page, { requireFocused: typeof inputText !== "function" });
  if (typeof inputText === "function") {
    await inputText(content, point, page);
  } else {
    await page.send("Input.insertText", { text: content });
  }
  return waitUntil("editor input", () => page.call(
    (expected) => {
      const observed = window.getEditorContent?.();
      return observed?.includes(expected) ? observed : null;
    },
    content,
  ), 5000).catch(async (error) => {
    const observed = await page.call(() => ({
      bridge: window.getEditorContent?.() ?? null,
      dom: globalThis.__deveVisibleElement?.(".cm-content")?.textContent ?? null,
      activeClass: document.activeElement?.className ?? null,
      activeEditable: document.activeElement?.getAttribute?.("contenteditable") ?? null,
      activeHostConnected: globalThis.__deveWebBridge
        ?.get?.("__deveEditorBootstrap")?.activeHost?.isConnected ?? null,
      activeHostMatchesVisible: globalThis.__deveWebBridge
        ?.get?.("__deveEditorBootstrap")?.activeHost
        === globalThis.__deveVisibleElement?.("[data-deve-editor-codemirror-host=true]"),
    })).catch(() => null);
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

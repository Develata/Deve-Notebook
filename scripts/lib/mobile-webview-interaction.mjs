export async function focusEditor(page) {
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
    visualViewportHeight: window.visualViewport?.height ?? null,
  }));
  console.log(`mobile-android-lifecycle: editor focus ${JSON.stringify({ point, focusState })}`);
  if (focusState.contentEditable !== "true") {
    throw new Error(`android_webview_editor_focus_unavailable: ${JSON.stringify(focusState)}`);
  }
}

export async function typeEditor(page, content, waitUntil) {
  await focusEditor(page);
  await page.send("Input.insertText", { text: content });
  await waitUntil("editor input", () => page.call(
    (expected) => window.getEditorContent?.().includes(expected) ?? false,
    content,
  ), 5000).catch((error) => {
    throw new Error(`android_webview_input_injection_unavailable: ${error.message}`);
  });
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

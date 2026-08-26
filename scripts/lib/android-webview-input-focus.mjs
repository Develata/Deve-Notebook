const WEBVIEW_INPUT_FOCUS_SETTLE_MS = 250;

export async function waitForCurrentWebViewInputFocus(page, waitUntil, timeout = 30000) {
  let matchedSince = null;
  let matchedDocument = null;
  await waitUntil("current Android WebView input focus settlement", async () => {
    let state;
    try {
      state = await page.call(() => ({
        documentTimeOrigin: performance.timeOrigin,
        visible: document.visibilityState === "visible",
        focused: document.hasFocus(),
        mobile: Boolean(document.querySelector('[data-deve-layout-mode="mobile"]')),
      }));
    } catch {
      matchedSince = null;
      matchedDocument = null;
      throw new Error("android_webview_input_focus_sample_failed");
    }
    if (!Number.isFinite(state?.documentTimeOrigin)
      || !state.visible || !state.focused || !state.mobile) {
      matchedSince = null;
      matchedDocument = null;
      return false;
    }
    if (matchedDocument !== state.documentTimeOrigin) {
      matchedDocument = state.documentTimeOrigin;
      matchedSince = Date.now();
      return false;
    }
    return Date.now() - matchedSince >= WEBVIEW_INPUT_FOCUS_SETTLE_MS;
  }, timeout);
}

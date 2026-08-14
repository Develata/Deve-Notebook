export async function readEditorMountObservation(page, expectedDocId = null) {
  return page.call((requiredDocId) => {
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
      : visibleHosts.filter((candidate) =>
        candidate.getAttribute("data-deve-editor-doc-id") === requiredDocId);
    const host = candidates.length === 1
      && (requiredDocId === null || visibleHosts.length === 1)
      ? candidates[0]
      : null;
    const codeHost = host
      ? [...host.querySelectorAll("[data-deve-editor-codemirror-host=true]")].find(isVisible)
      : null;
    const editor = codeHost
      ? [...codeHost.querySelectorAll(".cm-content")].find(isVisible)
      : null;
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
    const rect = editor?.getBoundingClientRect();
    const codeHostRect = codeHost?.getBoundingClientRect();
    const hostRect = host?.getBoundingClientRect();
    const visualViewport = window.visualViewport;
    const viewportHeight = visualViewport?.height ?? window.innerHeight;
    const viewportWidth = visualViewport?.width ?? window.innerWidth;
    const viewportTop = visualViewport?.offsetTop ?? 0;
    const viewportLeft = visualViewport?.offsetLeft ?? 0;
    const mobileLayout = document.querySelector?.('[data-deve-layout-mode="mobile"]') ?? null;
    const toolbar = document.querySelector?.('[data-deve-mobile-toolbar="accessory"]') ?? null;
    const toolbarRect = toolbar?.getBoundingClientRect?.() ?? null;
    const keyboardOffset = Number.parseInt(
      mobileLayout?.getAttribute("data-deve-keyboard-offset") ?? "0",
      10,
    );
    const nativeKeyboardOverlay = Number.parseInt(
      document.querySelector?.("[data-deve-native-keyboard-overlay]")
        ?.getAttribute("data-deve-native-keyboard-overlay") ?? "0",
      10,
    );
    const point = rect && codeHostRect && hostRect
      ? (() => {
          const left = Math.max(viewportLeft, rect.left, codeHostRect.left, hostRect.left);
          const right = Math.min(
            viewportLeft + viewportWidth,
            rect.right,
            codeHostRect.right,
            hostRect.right,
          );
          const top = Math.max(viewportTop, rect.top, codeHostRect.top, hostRect.top);
          const bottom = Math.min(
            viewportTop + viewportHeight,
            rect.bottom,
            codeHostRect.bottom,
            hostRect.bottom,
          );
          return right > left && bottom > top
            ? {
                x: left + Math.min(24, (right - left) / 2),
                y: top + Math.min(24, (bottom - top) / 2),
                devicePixelRatio: window.devicePixelRatio || 1,
              }
            : null;
        })()
      : null;
    const nativePresentation = globalThis.__DEVE_ANDROID_PRESENTATION__;
    let selectionIdentity = null;
    try {
      const rawSelection = globalThis.__deveWebBridge?.call?.("getEditorSelectionIdentity");
      const parsedSelection = typeof rawSelection === "string" ? JSON.parse(rawSelection) : null;
      if (
        Number.isSafeInteger(parsedSelection?.from)
        && Number.isSafeInteger(parsedSelection?.to)
        && Number.isSafeInteger(parsedSelection?.rangeCount)
      ) {
        selectionIdentity = {
          from: parsedSelection.from,
          to: parsedSelection.to,
          rangeCount: parsedSelection.rangeCount,
        };
      }
    } catch (_error) {
      selectionIdentity = null;
    }
    return {
      hostId,
      docId: host?.getAttribute("data-deve-editor-doc-id") ?? null,
      visibleHostCount: visibleHosts.length,
      openRequestId: host?.getAttribute("data-deve-editor-open-request-id") ?? null,
      point,
      viewportWidth: window.innerWidth,
      innerHeight: window.innerHeight,
      visualViewportHeight: viewportHeight,
      visualViewportOffsetTop: viewportTop,
      visualViewportOffsetLeft: viewportLeft,
      keyboardPresentation: mobileLayout?.getAttribute("data-deve-keyboard-presentation") ?? null,
      keyboardOffset: Number.isSafeInteger(keyboardOffset) && keyboardOffset >= 0
        ? keyboardOffset
        : null,
      nativeKeyboardOverlay: Number.isSafeInteger(nativeKeyboardOverlay)
        && nativeKeyboardOverlay >= 0
        ? nativeKeyboardOverlay
        : null,
      toolbarBottomGap: toolbarRect ? Math.max(0, window.innerHeight - toolbarRect.bottom) : null,
      repoId: mobileLayout?.getAttribute("data-deve-repo-id") ?? null,
      scopeNonce: mobileLayout?.getAttribute("data-deve-scope-nonce") ?? null,
      presentationGeneration: Number.isSafeInteger(nativePresentation?.generation)
        ? nativePresentation.generation
        : null,
      presentationEpoch: Number.isSafeInteger(nativePresentation?.epoch)
        ? nativePresentation.epoch
        : null,
      selectionIdentity,
      activeEditor: document.activeElement?.classList?.contains("cm-content") ?? false,
      bridgeReady: bootstrap?.editorBridgeReady === true,
      activeHostMatchesVisible: bootstrap?.activeHost === codeHost,
    };
  }, expectedDocId);
}

export function sameEditorLoadSession(before, after) {
  return before?.hostId != null
    && before.hostId === after?.hostId
    && before.openRequestId != null
    && before.openRequestId === after?.openRequestId
    && before.docId != null
    && before.docId === after?.docId
    && before.repoId != null
    && before.repoId === after?.repoId
    && before.scopeNonce != null
    && before.scopeNonce === after?.scopeNonce
    && before.presentationGeneration != null
    && before.presentationGeneration === after?.presentationGeneration;
}

export function sameEditorSelectionIdentity(before, after) {
  return before?.selectionIdentity != null
    && before.selectionIdentity.from === after?.selectionIdentity?.from
    && before.selectionIdentity.to === after?.selectionIdentity?.to
    && before.selectionIdentity.rangeCount === after?.selectionIdentity?.rangeCount;
}

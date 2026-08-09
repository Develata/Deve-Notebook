import { clickWebViewPoint } from "./android-webview-pointer.mjs";
import {
  closeMobileSidebar,
  openMobileSidebarView,
} from "./mobile-webview-interaction.mjs";

export function readAndroidMobileLayout() {
  return Boolean(globalThis.__deveVisibleElement('[data-deve-layout-mode="mobile"]'));
}

export function readAndroidSearchVisible() {
  return Boolean(globalThis.__deveVisibleElement("[data-deve-search-input=true]"));
}

export async function readExactCreateDocumentPointer(expectedPath) {
  const observe = () => {
    const candidates = [...document.querySelectorAll(
      '[data-deve-search-result-action="create-doc"]',
    )].filter((element) => {
      if (element.getAttribute("data-deve-search-result-create-target") !== expectedPath) {
        return false;
      }
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return style.display !== "none"
        && style.visibility !== "hidden"
        && rect.width > 0
        && rect.height > 0
        && rect.right > 0
        && rect.bottom > 0
        && rect.left < window.innerWidth
        && rect.top < window.innerHeight;
    });
    if (candidates.length !== 1) {
      return { kind: "not-unique", count: candidates.length };
    }
    const element = candidates[0];
    const rect = element.getBoundingClientRect();
    const point = {
      x: rect.left + Math.min(24, rect.width / 2),
      y: rect.top + Math.min(24, rect.height / 2),
    };
    const hit = document.elementFromPoint(point.x, point.y);
    if (!hit || (hit !== element && !element.contains(hit))) {
      return { kind: "occluded", count: 1 };
    }
    return {
      kind: "observed",
      count: 1,
      element,
      point,
      rect: { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
    };
  };
  const before = observe();
  if (before.kind !== "observed") return before;
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const after = observe();
  if (after.kind !== "observed") return after;
  const stable = before.element === after.element
    && Math.abs(before.rect.left - after.rect.left) < 0.5
    && Math.abs(before.rect.top - after.rect.top) < 0.5
    && Math.abs(before.rect.width - after.rect.width) < 0.5
    && Math.abs(before.rect.height - after.rect.height) < 0.5;
  return stable
    ? { kind: "ready", count: 1, point: after.point }
    : { kind: "moving", count: 1 };
}

export function armExactCreateDocumentClickObservation(expectedPath, point) {
  if (!point || !Number.isFinite(point.x) || !Number.isFinite(point.y)) {
    return { kind: "changed" };
  }
  const hit = document.elementFromPoint(point.x, point.y);
  const target = hit?.closest?.('[data-deve-search-result-action="create-doc"]') ?? null;
  const exactTargets = [...document.querySelectorAll(
    '[data-deve-search-result-action="create-doc"]',
  )].filter((element) =>
    element.getAttribute("data-deve-search-result-create-target") === expectedPath);
  if (exactTargets.length !== 1 || exactTargets[0] !== target) {
    return { kind: "changed" };
  }
  const previous = globalThis.__deveAndroidCreatePointerObservation;
  const token = Number.isSafeInteger(previous?.token) ? previous.token + 1 : 1;
  const observation = { token, clicked: false, clickState: null, target, listener: null };
  const listener = () => {
    if (globalThis.__deveAndroidCreatePointerObservation !== observation) return;
    const status = document.querySelector("[data-deve-sync-status]");
    observation.clicked = true;
    observation.clickState = {
      syncStatus: status?.getAttribute("data-deve-sync-status") ?? null,
      repoIdPresent: Boolean(status?.getAttribute("data-deve-repo-id")),
      scopeNonceRaw: status?.getAttribute("data-deve-scope-nonce") ?? null,
    };
  };
  observation.listener = listener;
  globalThis.__deveAndroidCreatePointerObservation = observation;
  target.addEventListener("click", listener, { capture: true, once: true });
  return { kind: "armed", token };
}

export function consumeExactCreateDocumentClickObservation(token) {
  const observation = globalThis.__deveAndroidCreatePointerObservation;
  if (!observation || observation.token !== token) return { kind: "missing", clicked: false };
  observation.target.removeEventListener("click", observation.listener, { capture: true });
  delete globalThis.__deveAndroidCreatePointerObservation;
  return {
    kind: "observed",
    clicked: observation.clicked,
    clickState: observation.clickState,
  };
}

export async function clickExactCreateDocument(page, path, tap = clickWebViewPoint) {
  const target = await page.call(readExactCreateDocumentPointer, path);
  if (target?.kind !== "ready" || target.count !== 1 || !target.point) {
    throw new Error(`exact Create target is not stable and visible: ${JSON.stringify(target)}`);
  }
  let armed = null;
  let pointerError = null;
  try {
    await tap(page, target.point, {
      beforePress: async () => {
        armed = await page.call(
          armExactCreateDocumentClickObservation,
          path,
          target.point,
        );
        if (armed?.kind !== "armed" || !Number.isSafeInteger(armed.token)) {
          throw new Error("exact Create target changed after pointer move");
        }
      },
    });
  } catch (error) {
    pointerError = error;
  }
  if (armed?.kind !== "armed") {
    if (pointerError) throw pointerError;
    throw new Error("Create pointer driver skipped before-press identity admission");
  }
  let observation;
  try {
    observation = await page.call(
      consumeExactCreateDocumentClickObservation,
      armed.token,
    );
  } catch (cleanupError) {
    throw new Error(
      `${pointerError?.message ?? "Create pointer observation cleanup failed"}; `
        + `observation_cleanup=${cleanupError.message}`,
    );
  }
  if (pointerError) {
    throw new Error(
      `${pointerError.message}; click_observation=${JSON.stringify(observation)}`,
    );
  }
  if (observation?.kind !== "observed" || observation.clicked !== true) {
    throw new Error(`exact Create pointer did not produce a DOM click: ${JSON.stringify(observation)}`);
  }
  return observation;
}

export function readAndroidDocumentCreateSurface(expectedPath, expectedDocId = null) {
  const isVisible = (element) => {
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return style.display !== "none"
      && style.visibility !== "hidden"
      && rect.width > 0
      && rect.height > 0;
  };
  const searchInput = globalThis.__deveVisibleElement("[data-deve-search-input=true]");
  const openResults = [...document.querySelectorAll(
    '[data-deve-search-result-action="open-doc"]',
  )].filter(isVisible);
  const exactOpenResults = openResults.filter((element) =>
    element.getAttribute("data-deve-search-result-title") === expectedPath);
  const createResults = [...document.querySelectorAll(
    '[data-deve-search-result-action="create-doc"]',
  )].filter(isVisible);
  const exactCreateResults = createResults.filter((element) =>
    element.getAttribute("data-deve-search-result-create-target") === expectedPath);
  const editorHosts = [...document.querySelectorAll("[data-deve-editor-host=true]")];
  const visibleEditorHosts = editorHosts.filter(isVisible);
  const exactEditorHosts = expectedDocId === null
    ? []
    : visibleEditorHosts.filter((element) =>
      element.getAttribute("data-deve-editor-doc-id") === expectedDocId);
  const status = document.querySelector("[data-deve-sync-status]");
  return {
    origin: location.origin,
    readyState: document.readyState,
    syncStatus: status?.getAttribute("data-deve-sync-status") ?? null,
    repoIdPresent: Boolean(status?.getAttribute("data-deve-repo-id")),
    scopeNonceRaw: status?.getAttribute("data-deve-scope-nonce") ?? null,
    searchVisible: Boolean(searchInput),
    searchQueryLength: typeof searchInput?.value === "string" ? searchInput.value.length : null,
    createResultCount: createResults.length,
    exactCreateResultCount: exactCreateResults.length,
    openResultCount: openResults.length,
    exactOpenResultCount: exactOpenResults.length,
    exactOpenDocIdCount: new Set(exactOpenResults.map((element) =>
      element.getAttribute("data-deve-search-result-doc-id")).filter(Boolean)).size,
    mobileDrawerOpen: document.querySelector('[data-deve-mobile-drawer="left"]')
      ?.getAttribute("data-deve-mobile-drawer-open") ?? null,
    editorHostCount: editorHosts.length,
    visibleEditorHostCount: visibleEditorHosts.length,
    exactEditorHostCount: exactEditorHosts.length,
    editorReadonly: visibleEditorHosts[0]
      ?.getAttribute("data-deve-editor-readonly") ?? null,
  };
}

export function clickExactCreatedDocumentAction(expectedPath) {
  const candidates = [...document.querySelectorAll(
    '[data-deve-search-result-action="open-doc"]',
  )].filter((element) => {
    if (element.getAttribute("data-deve-search-result-title") !== expectedPath) return false;
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return style.display !== "none"
      && style.visibility !== "hidden"
      && rect.width > 0
      && rect.height > 0;
  });
  if (candidates.length !== 1) return { kind: "not-unique", count: candidates.length };
  const docId = candidates[0].getAttribute("data-deve-search-result-doc-id");
  if (!docId) return { kind: "identity-missing", count: 1 };
  candidates[0].click();
  return { kind: "clicked", count: 1, docId };
}

export async function clickExactCreatedDocument(page, path) {
  const outcome = await page.call(clickExactCreatedDocumentAction, path);
  if (outcome?.kind !== "clicked" || outcome.count !== 1 || !outcome.docId) {
    throw new Error(
      `exact created document OpenDoc target is not unique and identified: ${JSON.stringify(outcome)}`,
    );
  }
  return outcome;
}

export function readExactCreatedEditorAdmission(expectedDocId) {
  const isVisible = (element) => {
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return style.display !== "none"
      && style.visibility !== "hidden"
      && rect.width > 0
      && rect.height > 0;
  };
  const hosts = [...document.querySelectorAll("[data-deve-editor-host=true]")]
    .filter(isVisible)
    .filter((host) => host.getAttribute("data-deve-editor-doc-id") === expectedDocId);
  const visibleHostCount = [...document.querySelectorAll("[data-deve-editor-host=true]")]
    .filter(isVisible).length;
  if (hosts.length !== 1 || visibleHostCount !== 1) {
    return { ready: false, exactEditorHostCount: hosts.length, visibleHostCount };
  }
  const host = hosts[0];
  const requestIdRaw = host.getAttribute("data-deve-editor-open-request-id");
  const requestId = /^\d+$/.test(requestIdRaw ?? "") ? Number(requestIdRaw) : null;
  const codeHost = [...host.querySelectorAll("[data-deve-editor-codemirror-host=true]")]
    .find(isVisible);
  const content = codeHost
    ? [...codeHost.querySelectorAll(".cm-content")].find(isVisible)
    : null;
  const bootstrap = globalThis.__deveWebBridge?.get?.("__deveEditorBootstrap");
  const ready = host.getAttribute("data-deve-editor-readonly") === "false"
    && Number.isSafeInteger(requestId)
    && requestId > 0
    && content?.getAttribute("contenteditable") === "true"
    && codeHost?.isConnected === true
    && bootstrap?.editorBridgeReady === true
    && bootstrap?.activeHost === codeHost;
  return {
    ready,
    exactEditorHostCount: 1,
    visibleHostCount,
    openRequestId: requestId,
    editorReadonly: host.getAttribute("data-deve-editor-readonly"),
    contentEditable: content?.getAttribute("contenteditable") ?? null,
    bridgeReady: bootstrap?.editorBridgeReady === true,
    activeHostMatched: bootstrap?.activeHost === codeHost,
  };
}

async function throwDocumentCreateFailure(page, path, phase, error, docId = null) {
  const state = await page.call(readAndroidDocumentCreateSurface, path, docId).catch(
    (diagnosticError) => ({ diagnosticError: diagnosticError.message }),
  );
  throw new Error(
    `android_document_create_${phase}: ${error.message}; state=${JSON.stringify(state)}`,
  );
}

export async function createAndSelectAndroidDocument(
  page,
  path,
  {
    waitUntil,
    click,
    fill,
    clickCreate = clickExactCreateDocument,
    clickExactOpen = clickExactCreatedDocument,
    openSidebar = openMobileSidebarView,
    closeSidebar = closeMobileSidebar,
  },
) {
  const mobile = await page.call(readAndroidMobileLayout);
  if (mobile) {
    await openSidebar(page, "explorer", { click, waitUntil });
  }
  await click(page, "[data-deve-new-doc-button=true]");
  await waitUntil("new document input", () => page.call(readAndroidSearchVisible));
  await fill(page, "[data-deve-search-input=true]", `+${path}`);
  await waitUntil(
    "exact Create document action",
    async () => {
      const target = await page.call(readExactCreateDocumentPointer, path);
      return target?.kind === "ready" && target.count === 1 ? target : null;
    },
    10000,
  ).catch((error) => throwDocumentCreateFailure(page, path, "create_identity", error));
  const pointerObservation = await clickCreate(page, path).catch(
    (error) => throwDocumentCreateFailure(page, path, "create_pointer", error),
  );
  await waitUntil(
    "create document action acknowledgement",
    async () => !(await page.call(readAndroidSearchVisible)),
    10000,
  ).catch((error) => throwDocumentCreateFailure(
    page,
    path,
    "action_ack",
    new Error(`${error.message}; pointer=${JSON.stringify(pointerObservation)}`),
  ));

  // Never repeat Create. Wait for the backend projection, then issue OpenDoc
  // for the one exact path and carry its typed doc identity into editor admission.
  await click(page, "[data-deve-new-doc-button=true]");
  await waitUntil("created document lookup input", () => page.call(readAndroidSearchVisible));
  await fill(page, "[data-deve-search-input=true]", path);
  const projection = await waitUntil(
    "exact created document projection",
    async () => {
      const current = await page.call(readAndroidDocumentCreateSurface, path);
      return current.exactOpenResultCount > 0 ? current : null;
    },
    30000,
  ).catch((error) => throwDocumentCreateFailure(page, path, "projection", error));
  if (projection.exactOpenResultCount !== 1 || projection.exactOpenDocIdCount !== 1) {
    await throwDocumentCreateFailure(
      page,
      path,
      "projection_identity",
      new Error("exact created document projection is not unique and identified"),
    );
  }
  const openOutcome = await clickExactOpen(page, path).catch(
    (error) => throwDocumentCreateFailure(page, path, "open_identity", error),
  );
  if (openOutcome?.kind !== "clicked" || !openOutcome.docId) {
    await throwDocumentCreateFailure(
      page,
      path,
      "open_identity",
      new Error("exact OpenDoc click did not return a typed document identity"),
    );
  }
  await waitUntil(
    "exact created document OpenDoc acknowledgement",
    async () => !(await page.call(readAndroidSearchVisible)),
    10000,
  ).catch((error) => throwDocumentCreateFailure(page, path, "open_ack", error, openOutcome.docId));
  if (mobile) {
    await closeSidebar(page, { click, waitUntil });
  }
  const editorAdmission = await waitUntil(
    "exact created document editor admission",
    async () => {
      const current = await page.call(readExactCreatedEditorAdmission, openOutcome.docId);
      return current.ready ? current : null;
    },
    30000,
  ).catch((error) => throwDocumentCreateFailure(
    page,
    path,
    "editor_identity",
    error,
    openOutcome.docId,
  ));
  return { ...projection, docId: openOutcome.docId, editorAdmission };
}

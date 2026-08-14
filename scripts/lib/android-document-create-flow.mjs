import {
  clickExactCreateDocument,
  readExactCreateDocumentPointer,
} from "./android-document-create-touch.mjs";
import { clickAndroidNewDocumentActionWhenAdmitted } from "./android-document-search-admission.mjs";
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
    newDocumentActionVisible: Boolean(
      globalThis.__deveVisibleElement("[data-deve-new-doc-button=true]"),
    ),
    mobileDrawerOpen: document.querySelector('[data-deve-mobile-drawer="left"]')
      ?.getAttribute("data-deve-mobile-drawer-open") ?? null,
    mobileExplorerActive: document.querySelector('[data-deve-mobile-sidebar-tab="explorer"]')
      ?.getAttribute("data-deve-mobile-sidebar-tab-active") ?? null,
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

async function openAndroidDocumentSearch(page, path, mobile, phase, options) {
  const { waitUntil, openSidebar } = options;
  if (mobile) {
    await openSidebar(page, "explorer", options).catch(
      (error) => throwDocumentCreateFailure(page, path, `${phase}_sidebar`, error),
    );
  }
  await waitUntil(
    `${phase} new document action admission`,
    async () => {
      const state = await page.call(clickAndroidNewDocumentActionWhenAdmitted);
      return state.clicked ? state : null;
    },
    30000,
  ).catch((error) => throwDocumentCreateFailure(page, path, `${phase}_admission`, error));
  await waitUntil(
    `${phase} document input`,
    () => page.call(readAndroidSearchVisible),
    10000,
  ).catch((error) => throwDocumentCreateFailure(page, path, `${phase}_input`, error));
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
    expectedWriterScope,
  },
) {
  if (!expectedWriterScope) {
    throw new Error("Android document Create requires an admitted writer scope");
  }
  const mobile = await page.call(readAndroidMobileLayout);
  const searchOptions = { waitUntil, click, openSidebar };
  await openAndroidDocumentSearch(page, path, mobile, "create", searchOptions);
  await fill(page, "[data-deve-search-input=true]", `+${path}`);
  await waitUntil(
    "exact Create document action",
    async () => {
      const target = await page.call(readExactCreateDocumentPointer, path);
      return target?.kind === "ready" && target.count === 1 ? target : null;
    },
    10000,
  ).catch((error) => throwDocumentCreateFailure(page, path, "create_identity", error));
  const pointerObservation = await clickCreate(page, path, expectedWriterScope).catch(
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
  await openAndroidDocumentSearch(page, path, mobile, "lookup", searchOptions);
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

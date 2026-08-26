import assert from "node:assert/strict";
import test from "node:test";

import {
  clickExactCreatedDocumentAction,
  createAndSelectAndroidDocument,
  readAndroidDocumentCreateSurface,
  readAndroidMobileLayout,
  readAndroidSearchVisible,
  readExactCreatedEditorAdmission,
} from "./lib/android-document-create-flow.mjs";
import { clickAndroidNewDocumentActionWhenAdmitted } from "./lib/android-document-search-admission.mjs";
import { readExactCreateDocumentPointer } from "./lib/android-document-create-touch.mjs";

const exactPath = "notes/exact.md";
const exactDocId = "00000000-0000-0000-0000-000000000007";

function documentCreateHarness({
  acknowledgeCreate = true,
  projectExactPath = true,
  exactProjectionCount = 1,
  editorReady = true,
  focusReady = true,
  mobile = false,
  deferPostCreateLookup = false,
  admitPostCreateLookup = true,
} = {}) {
  const state = {
    searchVisible: false,
    query: "",
    newDocumentClicks: 0,
    createClicks: 0,
    exactOpenClicks: 0,
    sidebarOpens: 0,
    sidebarCloses: 0,
    drawerOpen: false,
    postCreatePending: false,
    lookupAdmissionChecks: 0,
    inputFocusAdmissions: 0,
    actionOrder: [],
  };
  const surface = () => {
    const projected = projectExactPath && state.query && !state.query.startsWith("+")
      ? exactProjectionCount
      : 0;
    return {
      origin: "https://remote.test",
      readyState: "complete",
      syncStatus: "ready",
      repoIdPresent: true,
      scopeNonceRaw: "7",
      searchVisible: state.searchVisible,
      searchQueryLength: state.query.length,
      createResultCount: state.query.startsWith("+") ? 1 : 0,
      exactCreateResultCount: state.query === `+${exactPath}` ? 1 : 0,
      openResultCount: projected,
      exactOpenResultCount: projected,
      exactOpenDocIdCount: projected > 0 ? 1 : 0,
      mobileDrawerOpen: null,
      editorHostCount: 1,
      visibleEditorHostCount: 1,
      exactEditorHostCount: editorReady ? 1 : 0,
      editorReadonly: editorReady ? "false" : "true",
    };
  };
  const page = {
    async call(fn) {
      if (fn === readAndroidMobileLayout) return mobile;
      if (fn === clickAndroidNewDocumentActionWhenAdmitted) {
        state.lookupAdmissionChecks += 1;
        const ready = !state.postCreatePending
          || (admitPostCreateLookup && (!deferPostCreateLookup
            || (state.drawerOpen && state.lookupAdmissionChecks >= 2)));
        if (ready) {
          state.newDocumentClicks += 1;
          state.searchVisible = true;
        }
        return { clicked: ready };
      }
      if (fn === readAndroidSearchVisible) return state.searchVisible;
      if (fn === readExactCreateDocumentPointer) {
        return state.query === `+${exactPath}`
          ? { kind: "ready", count: 1, point: { x: 17, y: 23 } }
          : { kind: "not-unique", count: 0 };
      }
      if (fn === readAndroidDocumentCreateSurface) return surface();
      if (fn === readExactCreatedEditorAdmission) {
        return editorReady
          ? {
              ready: true,
              exactEditorHostCount: 1,
              openRequestId: 9,
              editorReadonly: "false",
              contentEditable: "true",
            }
          : { ready: false, exactEditorHostCount: 0 };
      }
      throw new Error(`unexpected document create page call: ${fn.name}`);
    },
  };
  const waitForState = async (label, predicate) => {
    for (let attempt = 0; attempt < 3; attempt += 1) {
      const value = await predicate();
      if (value) return value;
    }
    throw new Error(`timeout waiting for ${label}`);
  };
  const click = async (_page, selector) => {
    assert.fail(`unexpected direct document action click: ${selector}`);
  };
  const fill = async (_page, selector, value) => {
    assert.equal(selector, "[data-deve-search-input=true]");
    state.query = value;
  };
  const expectedWriterScope = { repoId: "repo-1", scopeNonce: 7 };
  const clickCreate = async (_page, path, writerScope) => {
    assert.equal(path, exactPath);
    assert.deepEqual(writerScope, expectedWriterScope);
    state.actionOrder.push("create");
    state.createClicks += 1;
    if (acknowledgeCreate) {
      state.searchVisible = false;
      state.postCreatePending = true;
      state.lookupAdmissionChecks = 0;
      if (deferPostCreateLookup) state.drawerOpen = false;
    }
  };
  const clickExactOpen = async (_page, path) => {
    assert.equal(path, exactPath);
    assert.equal(projectExactPath, true);
    state.exactOpenClicks += 1;
    state.searchVisible = false;
    return { kind: "clicked", count: 1, docId: exactDocId };
  };
  const openSidebar = async (_page, view) => {
    assert.equal(view, "explorer");
    state.sidebarOpens += 1;
    state.drawerOpen = true;
  };
  const closeSidebar = async () => {
    state.sidebarCloses += 1;
    state.drawerOpen = false;
  };
  const waitForInputFocus = async () => {
    state.actionOrder.push("focus");
    state.inputFocusAdmissions += 1;
    if (!focusReady) throw new Error("synthetic current WebView focus unavailable");
  };
  return {
    state,
    page,
    options: {
      waitUntil: waitForState,
      click,
      fill,
      clickCreate,
      clickExactOpen,
      openSidebar,
      closeSidebar,
      waitForInputFocus,
      expectedWriterScope,
    },
  };
}

test("Android document create binds one Create through doc-id to the exact editor", async () => {
  const harness = documentCreateHarness();
  const result = await createAndSelectAndroidDocument(harness.page, exactPath, harness.options);

  assert.equal(result.exactOpenResultCount, 1);
  assert.equal(result.docId, exactDocId);
  assert.equal(result.editorAdmission.openRequestId, 9);
  assert.equal(harness.state.newDocumentClicks, 2);
  assert.equal(harness.state.createClicks, 1);
  assert.equal(harness.state.exactOpenClicks, 1);
  assert.equal(harness.state.inputFocusAdmissions, 1);
  assert.deepEqual(harness.state.actionOrder, ["focus", "create"]);
});

test("Android document create fails before native touch without stable current WebView focus", async () => {
  const harness = documentCreateHarness({ focusReady: false });
  await assert.rejects(
    createAndSelectAndroidDocument(harness.page, exactPath, harness.options),
    /android_document_create_create_input_focus.*searchVisible.*true/,
  );
  assert.equal(harness.state.inputFocusAdmissions, 1);
  assert.equal(harness.state.createClicks, 0);
  assert.equal(harness.state.exactOpenClicks, 0);
});

test("Android document create does not retry Create without an action acknowledgement", async () => {
  const harness = documentCreateHarness({ acknowledgeCreate: false });
  await assert.rejects(
    createAndSelectAndroidDocument(harness.page, exactPath, harness.options),
    /android_document_create_action_ack.*searchVisible.*true/,
  );
  assert.equal(harness.state.createClicks, 1);
  assert.equal(harness.state.exactOpenClicks, 0);
});

test("Android document create closes the mobile drawer before editor admission", async () => {
  const harness = documentCreateHarness({ mobile: true });
  await createAndSelectAndroidDocument(harness.page, exactPath, harness.options);
  assert.equal(harness.state.sidebarOpens, 2);
  assert.equal(harness.state.sidebarCloses, 1);
});

test("Android document create reopens Explorer and waits for post-Create lookup admission", async () => {
  const harness = documentCreateHarness({ mobile: true, deferPostCreateLookup: true });
  await createAndSelectAndroidDocument(harness.page, exactPath, harness.options);
  assert.equal(harness.state.sidebarOpens, 2);
  assert.equal(harness.state.lookupAdmissionChecks, 2);
  assert.equal(harness.state.newDocumentClicks, 2);
  assert.equal(harness.state.createClicks, 1);
});

test("Android document create fails closed when post-Create lookup admission never returns", async () => {
  const harness = documentCreateHarness({ mobile: true, admitPostCreateLookup: false });
  await assert.rejects(
    createAndSelectAndroidDocument(harness.page, exactPath, harness.options),
    /android_document_create_lookup_admission/,
  );
  assert.equal(harness.state.newDocumentClicks, 1);
  assert.equal(harness.state.createClicks, 1);
  assert.equal(harness.state.exactOpenClicks, 0);
});

test("Android New Document admission clicks only one stable action in active mobile Explorer", async () => {
  let clicks = 0;
  const action = { click: () => { clicks += 1; } };
  let actionSamples = [{ click() {} }, { click() {} }];
  let drawerOpen = "true";
  let explorerActive = "true";
  const previous = {
    visible: globalThis.__deveVisibleElement,
    document: globalThis.document,
    animationFrame: globalThis.requestAnimationFrame,
  };
  globalThis.__deveVisibleElement = (selector) => {
    if (selector === "[data-deve-new-doc-button=true]") return actionSamples.shift() ?? action;
    return selector === '[data-deve-layout-mode="mobile"]' ? {} : null;
  };
  globalThis.document = { querySelector: (selector) => ({
    getAttribute: () => selector.includes("drawer") ? drawerOpen : explorerActive,
  }) };
  globalThis.requestAnimationFrame = (callback) => callback();
  try {
    assert.equal((await clickAndroidNewDocumentActionWhenAdmitted()).clicked, false);
    actionSamples = [action, action];
    drawerOpen = "false";
    assert.equal((await clickAndroidNewDocumentActionWhenAdmitted()).clicked, false);
    drawerOpen = "true";
    explorerActive = "false";
    assert.equal((await clickAndroidNewDocumentActionWhenAdmitted()).clicked, false);
    explorerActive = "true";
    assert.equal((await clickAndroidNewDocumentActionWhenAdmitted()).clicked, true);
    assert.equal(clicks, 1);
  } finally {
    if (previous.visible === undefined) delete globalThis.__deveVisibleElement;
    else globalThis.__deveVisibleElement = previous.visible;
    if (previous.document === undefined) delete globalThis.document;
    else globalThis.document = previous.document;
    if (previous.animationFrame === undefined) delete globalThis.requestAnimationFrame;
    else globalThis.requestAnimationFrame = previous.animationFrame;
  }
});

test("Android document create fails closed when the exact projection never arrives", async () => {
  const harness = documentCreateHarness({ projectExactPath: false });
  await assert.rejects(
    createAndSelectAndroidDocument(harness.page, exactPath, harness.options),
    /android_document_create_projection.*exactOpenResultCount.*0/,
  );
  assert.equal(harness.state.createClicks, 1);
  assert.equal(harness.state.exactOpenClicks, 0);
});

test("Android document create rejects duplicate exact-path OpenDoc projections", async () => {
  const harness = documentCreateHarness({ exactProjectionCount: 2 });
  await assert.rejects(
    createAndSelectAndroidDocument(harness.page, exactPath, harness.options),
    /android_document_create_projection_identity.*exactOpenResultCount.*2/,
  );
  assert.equal(harness.state.createClicks, 1);
  assert.equal(harness.state.exactOpenClicks, 0);
});

test("Android document create rejects an old writable editor after OpenDoc acknowledgement", async () => {
  const harness = documentCreateHarness({ editorReady: false });
  await assert.rejects(
    createAndSelectAndroidDocument(harness.page, exactPath, harness.options),
    /android_document_create_editor_identity.*exactEditorHostCount.*0/,
  );
  assert.equal(harness.state.exactOpenClicks, 1);
});

test("exact OpenDoc selection returns one typed doc identity atomically", () => {
  const originalDocument = globalThis.document;
  const originalGetComputedStyle = globalThis.getComputedStyle;
  const clicks = [];
  const result = (title, docId) => ({
    getAttribute: (name) => {
      if (name === "data-deve-search-result-title") return title;
      if (name === "data-deve-search-result-doc-id") return docId;
      return null;
    },
    getBoundingClientRect: () => ({ width: 100, height: 44 }),
    click: () => clicks.push(title),
  });
  globalThis.getComputedStyle = () => ({ display: "block", visibility: "visible" });
  globalThis.document = {
    querySelectorAll: () => [result("notes/other.md", "other"), result(exactPath, exactDocId)],
  };
  try {
    assert.deepEqual(
      clickExactCreatedDocumentAction(exactPath),
      { kind: "clicked", count: 1, docId: exactDocId },
    );
    assert.deepEqual(clicks, [exactPath]);
    globalThis.document.querySelectorAll = () => [
      result(exactPath, exactDocId),
      result(exactPath, "duplicate"),
    ];
    assert.deepEqual(
      clickExactCreatedDocumentAction(exactPath),
      { kind: "not-unique", count: 2 },
    );
    assert.deepEqual(clicks, [exactPath]);
  } finally {
    if (originalDocument === undefined) delete globalThis.document;
    else globalThis.document = originalDocument;
    if (originalGetComputedStyle === undefined) delete globalThis.getComputedStyle;
    else globalThis.getComputedStyle = originalGetComputedStyle;
  }
});

test("exact editor admission rejects the old doc and accepts the selected doc session", () => {
  const originalDocument = globalThis.document;
  const originalGetComputedStyle = globalThis.getComputedStyle;
  const originalBridge = globalThis.__deveWebBridge;
  const content = {
    getAttribute: (name) => name === "contenteditable" ? "true" : null,
    getBoundingClientRect: () => ({ width: 100, height: 44 }),
  };
  const codeHost = {
    isConnected: true,
    getBoundingClientRect: () => ({ width: 100, height: 80 }),
    querySelectorAll: () => [content],
  };
  const host = {
    getAttribute: (name) => ({
      "data-deve-editor-doc-id": exactDocId,
      "data-deve-editor-open-request-id": "9",
      "data-deve-editor-readonly": "false",
    })[name] ?? null,
    getBoundingClientRect: () => ({ width: 100, height: 100 }),
    querySelectorAll: () => [codeHost],
  };
  globalThis.getComputedStyle = () => ({ display: "block", visibility: "visible" });
  globalThis.document = { querySelectorAll: () => [host] };
  globalThis.__deveWebBridge = {
    get: () => ({ activeHost: codeHost, editorBridgeReady: true }),
  };
  try {
    assert.deepEqual(
      readExactCreatedEditorAdmission("old-doc-id"),
      { ready: false, exactEditorHostCount: 0, visibleHostCount: 1 },
    );
    assert.deepEqual(readExactCreatedEditorAdmission(exactDocId), {
      ready: true,
      exactEditorHostCount: 1,
      visibleHostCount: 1,
      openRequestId: 9,
      editorReadonly: "false",
      contentEditable: "true",
      bridgeReady: true,
      activeHostMatched: true,
    });
    const oldHost = {
      getAttribute: () => null,
      getBoundingClientRect: () => ({ width: 100, height: 100 }),
    };
    globalThis.document.querySelectorAll = () => [oldHost, host];
    assert.deepEqual(readExactCreatedEditorAdmission(exactDocId), {
      ready: false,
      exactEditorHostCount: 1,
      visibleHostCount: 2,
    });
  } finally {
    if (originalDocument === undefined) delete globalThis.document;
    else globalThis.document = originalDocument;
    if (originalGetComputedStyle === undefined) delete globalThis.getComputedStyle;
    else globalThis.getComputedStyle = originalGetComputedStyle;
    if (originalBridge === undefined) delete globalThis.__deveWebBridge;
    else globalThis.__deveWebBridge = originalBridge;
  }
});

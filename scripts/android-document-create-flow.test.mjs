import assert from "node:assert/strict";
import test from "node:test";

import {
  armExactCreateDocumentClickObservation,
  clickExactCreateDocument,
  clickExactCreatedDocumentAction,
  consumeExactCreateDocumentClickObservation,
  createAndSelectAndroidDocument,
  readAndroidDocumentCreateSurface,
  readAndroidMobileLayout,
  readAndroidSearchVisible,
  readExactCreateDocumentPointer,
  readExactCreatedEditorAdmission,
} from "./lib/android-document-create-flow.mjs";

const exactPath = "notes/exact.md";
const exactDocId = "00000000-0000-0000-0000-000000000007";

function documentCreateHarness({
  acknowledgeCreate = true,
  projectExactPath = true,
  exactProjectionCount = 1,
  editorReady = true,
  mobile = false,
} = {}) {
  const state = {
    searchVisible: false,
    query: "",
    newDocumentClicks: 0,
    createClicks: 0,
    exactOpenClicks: 0,
    sidebarOpens: 0,
    sidebarCloses: 0,
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
    const value = await predicate();
    if (value) return value;
    throw new Error(`timeout waiting for ${label}`);
  };
  const click = async (_page, selector) => {
    assert.equal(selector, "[data-deve-new-doc-button=true]");
    state.newDocumentClicks += 1;
    state.searchVisible = true;
  };
  const fill = async (_page, selector, value) => {
    assert.equal(selector, "[data-deve-search-input=true]");
    state.query = value;
  };
  const clickCreate = async (_page, path) => {
    assert.equal(path, exactPath);
    state.createClicks += 1;
    if (acknowledgeCreate) state.searchVisible = false;
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
  };
  const closeSidebar = async () => {
    state.sidebarCloses += 1;
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
  assert.equal(harness.state.sidebarOpens, 1);
  assert.equal(harness.state.sidebarCloses, 1);
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

function withCreateDom(elements, hit, run) {
  const originals = {
    document: globalThis.document,
    getComputedStyle: globalThis.getComputedStyle,
    window: globalThis.window,
    requestAnimationFrame: globalThis.requestAnimationFrame,
    __deveAndroidCreatePointerObservation:
      globalThis.__deveAndroidCreatePointerObservation,
  };
  globalThis.getComputedStyle = () => ({ display: "block", visibility: "visible" });
  globalThis.window = { innerWidth: 400, innerHeight: 800 };
  globalThis.requestAnimationFrame = (callback) => setImmediate(callback);
  globalThis.document = {
    querySelector: () => null,
    querySelectorAll: () => elements,
    elementFromPoint: () => hit,
  };
  return Promise.resolve(run()).finally(() => {
    for (const [name, value] of Object.entries(originals)) {
      if (value === undefined) delete globalThis[name];
      else globalThis[name] = value;
    }
  });
}

function createResult(target, { left = 10 } = {}) {
  const listeners = new Set();
  const element = {
    getAttribute: (name) => name === "data-deve-search-result-create-target" ? target : null,
    getBoundingClientRect: () => ({ left, top: 20, right: left + 100, bottom: 64, width: 100, height: 44 }),
    contains: () => false,
    addEventListener: (type, listener) => {
      if (type === "click") listeners.add(listener);
    },
    removeEventListener: (type, listener) => {
      if (type === "click") listeners.delete(listener);
    },
    emitClick: () => {
      for (const listener of [...listeners]) listener();
    },
  };
  element.closest = () => element;
  return element;
}

test("exact Create pointer ignores a stale result and requires a stable hit-tested target", async () => {
  const stale = createResult("Untitled.md");
  const exact = createResult(exactPath);
  await withCreateDom([stale, exact], exact, async () => {
    assert.deepEqual(
      await readExactCreateDocumentPointer(exactPath),
      { kind: "ready", count: 1, point: { x: 34, y: 42 } },
    );
  });
  await withCreateDom([exact, createResult(exactPath)], exact, async () => {
    assert.deepEqual(
      await readExactCreateDocumentPointer(exactPath),
      { kind: "not-unique", count: 2 },
    );
  });
  await withCreateDom([exact], stale, async () => {
    assert.deepEqual(
      await readExactCreateDocumentPointer(exactPath),
      { kind: "occluded", count: 1 },
    );
  });
});

test("exact Create pointer sends one native gesture only after identity admission", async () => {
  const page = {
    async call(fn, path) {
      if (fn === readExactCreateDocumentPointer) {
        assert.equal(path, exactPath);
        return { kind: "ready", count: 1, point: { x: 17, y: 23 } };
      }
      if (fn === armExactCreateDocumentClickObservation) {
        assert.equal(path, exactPath);
        return { kind: "armed", token: 7 };
      }
      assert.equal(fn, consumeExactCreateDocumentClickObservation);
      assert.equal(path, 7);
      return { kind: "observed", clicked: true, clickState: null };
    },
  };
  const taps = [];
  await clickExactCreateDocument(page, exactPath, async (tapPage, point, { beforePress }) => {
    taps.push({ tapPage, point });
    await beforePress();
  });
  assert.deepEqual(taps, [{ tapPage: page, point: { x: 17, y: 23 } }]);

  const changedPage = {
    async call(fn) {
      return fn === readExactCreateDocumentPointer
        ? { kind: "ready", count: 1, point: { x: 17, y: 23 } }
        : { kind: "changed" };
    },
  };
  await assert.rejects(
    clickExactCreateDocument(
      changedPage,
      exactPath,
      async (_tapPage, _point, { beforePress }) => beforePress(),
    ),
    /changed after pointer move/,
  );
  assert.equal(taps.includes("unexpected"), false);
});

test("exact Create production wiring emits the complete CDP pointer gesture", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    const sent = [];
    const page = {
      async call(fn, ...args) {
        return fn(...args);
      },
      async send(method, params) {
        sent.push({ method, params });
        if (params.type === "mouseReleased") exact.emitClick();
      },
    };

    const observation = await clickExactCreateDocument(page, exactPath);

    assert.equal(observation.clicked, true);
    assert.deepEqual(
      sent.map(({ params }) => ({
        type: params.type,
        button: params.button,
        buttons: params.buttons,
      })),
      [
        { type: "mouseMoved", button: "none", buttons: 0 },
        { type: "mousePressed", button: "left", buttons: 1 },
        { type: "mouseReleased", button: "left", buttons: 0 },
      ],
    );
  });
});

test("exact Create pointer cleans observation after a committed-unknown release", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    const page = {
      async call(fn, ...args) {
        return fn(...args);
      },
      async send(_method, params) {
        if (params.type !== "mouseReleased") return;
        exact.emitClick();
        throw new Error("release response lost");
      },
    };

    await assert.rejects(
      clickExactCreateDocument(page, exactPath),
      /release response lost; click_observation=.*"clicked":true/,
    );
    assert.equal(globalThis.__deveAndroidCreatePointerObservation, undefined);
  });
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

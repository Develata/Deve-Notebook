import assert from "node:assert/strict";
import test from "node:test";

import {
  editorFocusMatchesMode,
  parsePendingAckCount,
  readSourceControlCommitState,
  sameEditorLoadSession,
  sourceControlCommitAcknowledged,
  sourceControlCommitReady,
  typeAndroidEditorText,
  typeAndroidTextField,
} from "./lib/mobile-webview-interaction.mjs";
import { focusEditor } from "./lib/mobile-webview-interaction.mjs";

test("writable editor focus requires a contenteditable CodeMirror surface", () => {
  assert.equal(editorFocusMatchesMode("true", true), true);
  assert.equal(editorFocusMatchesMode("true", true, false), false);
  assert.equal(editorFocusMatchesMode("false", true), false);
  assert.equal(editorFocusMatchesMode(null, true), false);
});

test("read-only editor focus rejects a writable CodeMirror surface", () => {
  assert.equal(editorFocusMatchesMode("false", false), true);
  assert.equal(editorFocusMatchesMode(null, false), true);
  assert.equal(editorFocusMatchesMode(null, false, false), false);
  assert.equal(editorFocusMatchesMode("true", false), false);
});

test("pending acknowledgement count fails closed on missing or invalid markers", () => {
  assert.equal(parsePendingAckCount("0"), 0);
  assert.equal(parsePendingAckCount("37"), 37);
  for (const invalid of [null, undefined, "", "01", "-1", "1.5", "NaN"]) {
    assert.throws(() => parsePendingAckCount(invalid), /android_pending_ack_marker_invalid/);
  }
});

test("editor load-session comparison requires the same host and OpenDoc request", () => {
  const before = { hostId: 7, openRequestId: "11" };
  assert.equal(sameEditorLoadSession(before, { hostId: 7, openRequestId: "11" }), true);
  assert.equal(sameEditorLoadSession(before, { hostId: 8, openRequestId: "11" }), false);
  assert.equal(sameEditorLoadSession(before, { hostId: 7, openRequestId: "12" }), false);
  assert.equal(sameEditorLoadSession(before, { hostId: null, openRequestId: "11" }), false);
});

test("editor focus targets the CodeMirror surface without a desktop pointer event", async () => {
  const point = {
    x: 24,
    y: 32,
    devicePixelRatio: 2,
    rect: { left: 0, top: 8, width: 100, height: 80 },
  };
  const results = [
    point,
    {
      tag: "DIV",
      className: "cm-content",
      contentEditable: "true",
      activeEditor: true,
      visualViewportHeight: 600,
    },
  ];
  const page = {
    async call() {
      return results.shift();
    },
    async send() {
      throw new Error("focus must not dispatch a desktop pointer event on Android");
    },
  };

  assert.deepEqual(await focusEditor(page), point);
});

test("native input driver can recover focus from a returned editor point", async () => {
  const point = {
    x: 24,
    y: 32,
    devicePixelRatio: 2,
    rect: { left: 0, top: 8, width: 100, height: 80 },
  };
  const results = [
    point,
    {
      tag: "BODY",
      className: "",
      contentEditable: null,
      activeEditor: false,
      visualViewportHeight: 400,
    },
  ];
  const page = {
    async call() {
      return results.shift();
    },
  };

  assert.deepEqual(await focusEditor(page, { requireFocused: false }), point);
});

test("read-only input proof rejects focus that fell back to the document body", async () => {
  const results = [
    {
      x: 24,
      y: 32,
      devicePixelRatio: 2,
      rect: { left: 0, top: 8, width: 100, height: 80 },
    },
    {
      tag: "BODY",
      className: "",
      contentEditable: null,
      activeEditor: false,
      visualViewportHeight: 400,
    },
  ];
  const page = {
    async call() {
      return results.shift();
    },
  };

  await assert.rejects(
    focusEditor(page, { writable: false }),
    /android_webview_editor_focus_mode_mismatch/,
  );
});

test("Android text field input retries remounted fields before inserting", async () => {
  const point = { x: 20, y: 30, devicePixelRatio: 2 };
  const results = [point, false, point, true];
  const taps = [];
  const sends = [];
  const page = {
    async call() {
      return results.shift();
    },
    async send(method, params) {
      sends.push({ method, params });
    },
  };

  await typeAndroidTextField(page, "textarea", "commit", {
    tap: async (nextPoint) => taps.push(nextPoint),
    delay: async () => {},
  });

  assert.equal(taps.length, 2);
  assert.deepEqual(sends, [{ method: "Input.insertText", params: { text: "commit" } }]);
});

test("Android editor input binds expected doc-id through native input connection", async () => {
  const point = { x: 24, y: 32 };
  const expectedDocId = "00000000-0000-0000-0000-000000000007";
  const order = [];
  let callCount = 0;
  const page = {
    async call() {
      callCount += 1;
      if (callCount === 1) return point;
      order.push("focus");
      return {
        tag: "DIV",
        className: "cm-content",
        contentEditable: "true",
        activeEditor: true,
        identityMatched: true,
        visualViewportHeight: 600,
      };
    },
  };

  const observation = {
    hostId: 7,
    openRequestId: "11",
    point,
    bridgeReady: true,
    activeHostMatchesVisible: true,
    activeEditor: true,
  };

  await typeAndroidEditorText(page, "Android input", {
    tap: async (_page, observedPoint) => {
      assert.deepEqual(observedPoint, point);
      order.push("tap");
    },
    delay: async (milliseconds) => order.push(`delay:${milliseconds}`),
    waitForWritableEditor: async (observedPage, requiredDocId) => {
      assert.equal(observedPage, page);
      assert.equal(requiredDocId, expectedDocId);
      order.push("writable");
    },
    observeEditor: async (_page, requiredDocId) => {
      assert.equal(requiredDocId, expectedDocId);
      return observation;
    },
    inputText: async (value) => order.push(`input:${value}`),
    expectedDocId,
  });

  assert.deepEqual(order, [
    "writable",
    "tap",
    "delay:250",
    "writable",
    "focus",
    "delay:300",
    "writable",
    "input:Android input",
  ]);
});

test("Android editor input fails closed after bounded focus retries", async () => {
  const point = { x: 24, y: 32 };
  let callCount = 0;
  let taps = 0;
  let writableChecks = 0;
  let inputs = 0;
  const page = {
    async call() {
      callCount += 1;
      return callCount % 2 === 1
        ? point
        : {
            tag: "BODY",
            className: "",
            contentEditable: null,
            activeEditor: false,
            visualViewportHeight: 600,
          };
    },
  };

  const observation = {
    hostId: 7,
    openRequestId: "11",
    point,
    bridgeReady: true,
    activeHostMatchesVisible: true,
  };

  await assert.rejects(
    typeAndroidEditorText(page, "must not type", {
      tap: async () => {
        taps += 1;
      },
      delay: async () => {},
      waitForWritableEditor: async () => {
        writableChecks += 1;
      },
      observeEditor: async () => observation,
      inputText: async () => {
        inputs += 1;
      },
    }),
    /android_webview_editor_focus_mode_mismatch/,
  );

  assert.equal(taps, 3);
  assert.equal(writableChecks, 6);
  assert.equal(inputs, 0);
});

test("Android editor input reacquires a remounted editor before typing once", async () => {
  const oldPoint = { x: 24, y: 32 };
  const newPoint = { x: 48, y: 64 };
  const observations = [
    {
      hostId: 7,
      openRequestId: "11",
      point: oldPoint,
      bridgeReady: true,
      activeHostMatchesVisible: true,
    },
    {
      hostId: 8,
      openRequestId: "12",
      point: newPoint,
      bridgeReady: true,
      activeHostMatchesVisible: true,
      activeEditor: true,
    },
    {
      hostId: 8,
      openRequestId: "12",
      point: newPoint,
      bridgeReady: true,
      activeHostMatchesVisible: true,
      activeEditor: true,
    },
    {
      hostId: 8,
      openRequestId: "12",
      point: newPoint,
      bridgeReady: true,
      activeHostMatchesVisible: true,
      activeEditor: true,
    },
    {
      hostId: 8,
      openRequestId: "12",
      point: newPoint,
      bridgeReady: true,
      activeHostMatchesVisible: true,
      activeEditor: true,
    },
    {
      hostId: 8,
      openRequestId: "12",
      point: newPoint,
      bridgeReady: true,
      activeHostMatchesVisible: true,
      activeEditor: true,
    },
  ];
  const taps = [];
  const inputs = [];
  let callCount = 0;
  const page = {
    async call() {
      callCount += 1;
      return callCount === 1
        ? newPoint
        : {
            tag: "DIV",
            className: "cm-content",
            contentEditable: "true",
            activeEditor: true,
            visualViewportHeight: 600,
          };
    },
  };

  await typeAndroidEditorText(page, "after remount", {
    tap: async (_page, point) => taps.push(point),
    delay: async () => {},
    waitForWritableEditor: async () => {},
    observeEditor: async () => observations.shift(),
    inputText: async (value) => inputs.push(value),
  });

  assert.deepEqual(taps, [oldPoint, newPoint]);
  assert.deepEqual(inputs, ["after remount"]);
  assert.equal(observations.length, 0);
});

test("Android text field can delegate insertion to a WebView input driver", async () => {
  const inserted = [];
  const page = {
    async call() {
      return { x: 20, y: 30, devicePixelRatio: 2 };
    },
    async send() {
      throw new Error("CDP text insertion must stay unused when a native driver is present");
    },
  };

  await typeAndroidTextField(page, "textarea", "native commit", {
    tap: async () => {},
    delay: async () => {},
    inputText: async (value) => inserted.push(value),
  });

  assert.deepEqual(inserted, ["native commit"]);
});

test("Source Control commit readiness requires confirmed changes and the bound message", () => {
  const ready = {
    message: "android lifecycle",
    fieldDisabled: false,
    buttonDisabled: false,
    confirmedCount: 1,
  };

  assert.equal(sourceControlCommitReady(ready, "android lifecycle"), true);
  assert.equal(sourceControlCommitReady({ ...ready, confirmedCount: 0 }, "android lifecycle"), false);
  assert.equal(sourceControlCommitReady({ ...ready, message: "" }, "android lifecycle"), false);
  assert.equal(sourceControlCommitReady({ ...ready, buttonDisabled: true }, "android lifecycle"), false);
});

test("Source Control commit state counts the stable confirmed row marker", async () => {
  let observedSelector = null;
  const button = { disabled: false, title: "Commit confirmed changes" };
  const panel = {
    parentElement: {
      querySelectorAll(selector) {
        observedSelector = selector;
        return [{}, {}];
      },
    },
    querySelector() {
      return button;
    },
  };
  const field = {
    value: "android lifecycle",
    disabled: false,
    closest() {
      return panel;
    },
  };
  const previousVisibleElement = globalThis.__deveVisibleElement;
  globalThis.__deveVisibleElement = () => field;

  try {
    const state = await readSourceControlCommitState({
      async call(callback) {
        return callback();
      },
    });
    assert.equal(state.confirmedCount, 2);
    assert.equal(
      observedSelector,
      '[data-deve-sc-section-body="confirmed-ledger"] '
        + '[data-deve-mobile-touch-target="source-control-change-row"]',
    );
  } finally {
    if (previousVisibleElement === undefined) {
      delete globalThis.__deveVisibleElement;
    } else {
      globalThis.__deveVisibleElement = previousVisibleElement;
    }
  }
});

test("Source Control commit acknowledgement requires the confirmed set to clear", () => {
  assert.equal(sourceControlCommitAcknowledged({ message: "", confirmedCount: 0 }), true);
  assert.equal(sourceControlCommitAcknowledged({ message: "", confirmedCount: 1 }), false);
  assert.equal(sourceControlCommitAcknowledged({ message: "pending", confirmedCount: 0 }), false);
});

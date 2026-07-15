import assert from "node:assert/strict";
import test from "node:test";

import { CdpPage } from "./lib/android-webview-cdp.mjs";
import {
  clickWebViewPoint,
  editorFocusMatchesMode,
  parsePendingAckCount,
  readSourceControlCommitState,
  sameEditorLoadSession,
  sourceControlCommitAcknowledged,
  sourceControlCommitReady,
  typeAndroidTextField,
} from "./lib/mobile-webview-interaction.mjs";
import { focusEditor } from "./lib/mobile-webview-interaction.mjs";

class SilentSocket extends EventTarget {
  static CLOSED = 3;

  constructor() {
    super();
    this.readyState = 0;
    queueMicrotask(() => {
      this.readyState = 1;
      this.dispatchEvent(new Event("open"));
    });
  }

  send() {}

  close() {
    this.readyState = SilentSocket.CLOSED;
    this.dispatchEvent(new Event("close"));
  }
}

test("Android CDP discovery bounds Runtime.enable and retires its waiter", async () => {
  const originalWebSocket = globalThis.WebSocket;
  const observed = [];
  globalThis.WebSocket = SilentSocket;
  const withDeadline = async (label, promise, limit) => {
    observed.push({ label, limit });
    if (label === "Runtime.enable") throw new Error("synthetic command timeout");
    return promise;
  };

  try {
    await assert.rejects(
      CdpPage.connect("ws://android.test/page", withDeadline),
      /synthetic command timeout/,
    );
  } finally {
    if (originalWebSocket === undefined) delete globalThis.WebSocket;
    else globalThis.WebSocket = originalWebSocket;
  }

  assert.deepEqual(
    observed.find(({ label }) => label === "Runtime.enable"),
    { label: "Runtime.enable", limit: 10_000 },
  );
});

test("timed-out Android CDP commands cannot retain pending response waiters", async () => {
  const socket = new SilentSocket();
  const page = new CdpPage(socket, async (label, _promise, limit) => {
    assert.equal(label, "Runtime.evaluate");
    assert.equal(limit, 25);
    throw new Error("synthetic command timeout");
  });

  await assert.rejects(
    page.send("Runtime.evaluate", {}, 25),
    /synthetic command timeout/,
  );
  assert.equal(page.pending.size, 0);
});

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

test("WebView point click dispatches one complete primary-button gesture", async () => {
  const sent = [];
  const page = {
    async send(method, params) {
      sent.push({ method, params });
    },
  };

  await clickWebViewPoint(page, { x: 24, y: 32 });

  assert.deepEqual(sent, [
    {
      method: "Input.dispatchMouseEvent",
      params: { type: "mousePressed", x: 24, y: 32, button: "left", clickCount: 1 },
    },
    {
      method: "Input.dispatchMouseEvent",
      params: { type: "mouseReleased", x: 24, y: 32, button: "left", clickCount: 1 },
    },
  ]);
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

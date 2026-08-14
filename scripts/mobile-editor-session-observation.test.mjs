import assert from "node:assert/strict";
import test from "node:test";

import {
  readEditorMountObservation,
  sameEditorLoadSession,
  sameEditorSelectionIdentity,
} from "./lib/mobile-editor-session-observation.mjs";

test("editor load-session and selection identities have separate responsibilities", () => {
  const before = {
    hostId: 7,
    openRequestId: "11",
    docId: "doc-1",
    repoId: "repo-1",
    scopeNonce: "9",
    presentationGeneration: 3,
    selectionIdentity: { from: 5, to: 5, rangeCount: 1 },
  };
  assert.equal(sameEditorLoadSession(before, { ...before }), true);
  for (const changed of [
    { hostId: 8 },
    { openRequestId: "12" },
    { docId: "doc-2" },
    { repoId: "repo-2" },
    { scopeNonce: "10" },
    { presentationGeneration: 4 },
  ]) {
    assert.equal(sameEditorLoadSession(before, { ...before, ...changed }), false);
  }
  const movedCaret = {
    ...before,
    selectionIdentity: { from: 6, to: 6, rangeCount: 1 },
  };
  assert.equal(sameEditorLoadSession(before, movedCaret), true);
  assert.equal(sameEditorSelectionIdentity(before, movedCaret), false);
  assert.equal(sameEditorSelectionIdentity(before, { ...before }), true);
});

test("editor observation clamps native tap to the offset visual viewport intersection", async () => {
  const page = {
    call: async (fn, expectedDocId) => {
      const previous = globalThis.document;
      const previousWindow = globalThis.window;
      const rect = (left, top, right, bottom) => ({
        left, top, right, bottom, width: right - left, height: bottom - top,
      });
      const editor = { getBoundingClientRect: () => rect(30, -7000, 390, 900) };
      const codeHost = {
        getBoundingClientRect: () => rect(20, 140, 392, 900),
        querySelectorAll: () => [editor],
      };
      const host = {
        getBoundingClientRect: () => rect(0, 120, 392, 900),
        getAttribute: (key) => key === "data-deve-editor-doc-id" ? expectedDocId : "1",
        querySelectorAll: () => [codeHost],
      };
      globalThis.window = {
        innerWidth: 392,
        innerHeight: 872,
        devicePixelRatio: 2.75,
        visualViewport: { height: 400, width: 300, offsetTop: 300, offsetLeft: 40 },
        __deveWebBridge: undefined,
      };
      globalThis.document = {
        activeElement: editor,
        querySelectorAll: () => [host],
      };
      globalThis.getComputedStyle = () => ({ display: "block", visibility: "visible" });
      try {
        return fn(expectedDocId);
      } finally {
        globalThis.document = previous;
        globalThis.window = previousWindow;
      }
    },
  };
  const observed = await readEditorMountObservation(page, "doc-1");
  assert.deepEqual(observed.point, { x: 64, y: 324, devicePixelRatio: 2.75 });
});

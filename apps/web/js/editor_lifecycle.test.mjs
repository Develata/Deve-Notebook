import assert from "node:assert/strict";
import test from "node:test";

import {
  activateEditorMount,
  captureGestureEditorSelection,
  destroyOwnedEditorMount,
  retireGestureEditorSelection,
  settleGestureEditorSelection,
} from "./editor_lifecycle.js";

function fakeView(name) {
  return {
    name,
    destroyed: 0,
    destroy() {
      this.destroyed += 1;
    },
  };
}

test("stale cleanup cannot destroy a replacement editor mount", () => {
  const state = { activeHost: null, activeView: null };
  const oldHost = {};
  const newHost = {};
  const oldView = fakeView("old");
  const newView = fakeView("new");

  activateEditorMount(state, oldHost, () => oldView);
  activateEditorMount(state, newHost, () => newView);

  assert.equal(oldView.destroyed, 1, "replacement must retire the previous view");
  assert.equal(destroyOwnedEditorMount(state, oldHost), false);
  assert.equal(newView.destroyed, 0, "stale cleanup must preserve the current view");
  assert.equal(state.activeHost, newHost);
  assert.equal(state.activeView, newView);

  assert.equal(destroyOwnedEditorMount(state, newHost), true);
  assert.equal(newView.destroyed, 1);
  assert.equal(state.activeHost, null);
  assert.equal(state.activeView, null);
});

test("failed replacement leaves no falsely active editor mount", () => {
  const state = { activeHost: null, activeView: null };
  const oldView = fakeView("old");
  activateEditorMount(state, {}, () => oldView);

  assert.throws(
    () => activateEditorMount(state, {}, () => {
      throw new Error("mount failed");
    }),
    /mount failed/,
  );

  assert.equal(oldView.destroyed, 1);
  assert.equal(state.activeHost, null);
  assert.equal(state.activeView, null);
});

test("throwing teardown still clears mount ownership", () => {
  const state = { activeHost: null, activeView: null };
  const host = {};
  activateEditorMount(state, host, () => ({
    destroy() {
      throw new Error("destroy failed");
    },
  }));

  assert.throws(() => destroyOwnedEditorMount(state, host), /destroy failed/);
  assert.equal(state.activeHost, null);
  assert.equal(state.activeView, null);
});

test("disconnected stale mount cannot retire the current editor", () => {
  const state = { activeHost: null, activeView: null };
  const currentHost = { isConnected: true };
  const currentView = fakeView("current");
  activateEditorMount(state, currentHost, () => currentView);

  assert.throws(
    () => activateEditorMount(state, { isConnected: false }, () => fakeView("stale")),
    /disconnected/,
  );
  assert.equal(currentView.destroyed, 0);
  assert.equal(state.activeHost, currentHost);
  assert.equal(state.activeView, currentView);
});

function selectionView(selection, docLength = 12) {
  return {
    state: { selection, doc: { length: docLength } },
    dispatched: [],
    dispatch(spec) {
      this.dispatched.push(spec);
    },
  };
}

test("gesture selection token restores the exact selection once on the same editor", () => {
  retireGestureEditorSelection();
  const selection = { ranges: [{ anchor: 2, head: 7 }], mainIndex: 0 };
  const view = selectionView(selection);
  const state = { activeView: view };
  const token = captureGestureEditorSelection(state);

  assert.equal(Number.isSafeInteger(token), true);
  assert.ok(token > 0);
  view.state.selection = { ranges: [{ anchor: 8, head: 8 }], mainIndex: 0 };
  assert.equal(settleGestureEditorSelection(state, token, true), true);
  assert.deepEqual(view.dispatched, [{ selection }]);
  assert.equal(settleGestureEditorSelection(state, token, true), false);
});

test("gesture selection token fails closed after editor or document replacement", () => {
  retireGestureEditorSelection();
  const original = selectionView({ ranges: [{ anchor: 1, head: 1 }], mainIndex: 0 });
  const state = { activeView: original };
  const replacedToken = captureGestureEditorSelection(state);
  state.activeView = selectionView(original.state.selection);
  assert.equal(settleGestureEditorSelection(state, replacedToken, true), false);
  assert.deepEqual(original.dispatched, []);

  state.activeView = original;
  const changedToken = captureGestureEditorSelection(state);
  original.state.doc.length += 1;
  assert.equal(settleGestureEditorSelection(state, changedToken, true), false);
  assert.deepEqual(original.dispatched, []);

  const sameLengthToken = captureGestureEditorSelection(state);
  original.state.doc = { length: original.state.doc.length };
  assert.equal(settleGestureEditorSelection(state, sameLengthToken, true), false);
  assert.deepEqual(original.dispatched, []);
});

test("retiring a gesture selection token never dispatches a selection", () => {
  retireGestureEditorSelection();
  const view = selectionView({ ranges: [{ anchor: 3, head: 3 }], mainIndex: 0 });
  const state = { activeView: view };
  const token = captureGestureEditorSelection(state);

  assert.equal(settleGestureEditorSelection(state, token, false), true);
  assert.deepEqual(view.dispatched, []);
  assert.equal(captureGestureEditorSelection({ activeView: null }), null);
});

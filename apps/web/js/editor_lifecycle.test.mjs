import assert from "node:assert/strict";
import test from "node:test";

import {
  activateEditorMount,
  destroyOwnedEditorMount,
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

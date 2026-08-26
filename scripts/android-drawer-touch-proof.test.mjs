import assert from "node:assert/strict";
import test from "node:test";
import {
  beginTouchDeliveryProbe,
  classifyAndroidDrawerGestureDelivery,
  selectNonInteractiveSwipePoints,
  shouldRetryAndroidDrawerGestureDelivery,
  takeTouchDeliveryProbe,
} from "./lib/android-drawer-touch-proof.mjs";

const expectedLeftDelivery = {
  startXCss: 40,
  startYCss: 100,
  endXCss: 120,
  endYCss: 100,
  direction: 1,
};

const completeLeftDelivery = (identifier = 7) => [
  { type: "touchstart", identifier, x: 40, y: 100, touchCount: 1 },
  { type: "touchend", identifier, x: 120, y: 100, touchCount: 0 },
];

test("drawer delivery classifier binds identifier, coordinates, direction, and retry categories", () => {
  assert.equal(classifyAndroidDrawerGestureDelivery([], expectedLeftDelivery), "missing");
  assert.equal(classifyAndroidDrawerGestureDelivery([
    { type: "touchstart", identifier: 7, x: 40, y: 100, touchCount: 1 },
    { type: "touchcancel", identifier: 7, x: 40, y: 100, touchCount: 0 },
  ], expectedLeftDelivery), "cancelled");
  assert.equal(classifyAndroidDrawerGestureDelivery(completeLeftDelivery(), expectedLeftDelivery), "complete");
  assert.equal(classifyAndroidDrawerGestureDelivery([
    completeLeftDelivery()[0],
    { ...completeLeftDelivery()[1], identifier: 8 },
  ], expectedLeftDelivery), "invalid");
  assert.equal(classifyAndroidDrawerGestureDelivery([
    { ...completeLeftDelivery()[0], x: 300 },
    completeLeftDelivery()[1],
  ], expectedLeftDelivery), "invalid");
  assert.equal(classifyAndroidDrawerGestureDelivery([
    completeLeftDelivery()[0],
    { ...completeLeftDelivery()[1], x: 10 },
  ], expectedLeftDelivery), "invalid");
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("missing", 1, 3), true);
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("cancelled", 2, 3), true);
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("complete", 1, 3), false);
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("invalid", 1, 3), false);
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("missing", 3, 3), false);
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("missing", 1), false);
});

test("drawer hit testing rejects action controls and requires the marked Work Edit surface", async () => {
  const mobileRoot = {};
  const workEditRoot = {};
  const makePage = (blocked, workEdit = true, editorContent = workEdit) => ({
    async call(callback, ...args) {
      const previousWindow = globalThis.window;
      const previousDocument = globalThis.document;
      globalThis.window = { innerHeight: 1000 };
      globalThis.document = {
        elementFromPoint() {
          return {
            tagName: "DIV",
            closest(selector) {
              if (selector === '[data-deve-layout-mode="mobile"]') return mobileRoot;
              if (selector.includes("data-deve-mobile-work-edit-swipe-surface")) {
                if (selector.includes(".cm-content")) return editorContent ? workEditRoot : null;
                return workEdit ? workEditRoot : null;
              }
              return blocked && selector.includes("button") ? this : null;
            },
          };
        },
      };
      try {
        return callback(...args);
      } finally {
        globalThis.window = previousWindow;
        globalThis.document = previousDocument;
      }
    },
  });
  assert.deepEqual(await selectNonInteractiveSwipePoints(makePage(true), 40, [0.5]), []);
  assert.deepEqual(await selectNonInteractiveSwipePoints(makePage(false), 40, [0.5]), [
    { yCss: 500, targetTag: "div" },
  ]);
  const required = '[data-deve-mobile-work-edit-swipe-surface="true"] .cm-content';
  assert.deepEqual(
    await selectNonInteractiveSwipePoints(makePage(false, false), 200, [0.5], required),
    [],
  );
  assert.deepEqual(
    await selectNonInteractiveSwipePoints(makePage(false, true, false), 200, [0.5], required),
    [],
  );
  assert.deepEqual(
    await selectNonInteractiveSwipePoints(makePage(false, true, true), 200, [0.5], required),
    [{ yCss: 500, targetTag: "div" }],
  );
});

test("touch delivery probe aborts listeners and deletes state on success or read failure", async () => {
  const listeners = [];
  const previousWindow = globalThis.window;
  globalThis.window = {
    addEventListener(type, callback, options) {
      listeners.push({ type, callback, signal: options.signal });
    },
  };
  const page = { call: async (callback, ...args) => callback(...args) };
  try {
    await beginTouchDeliveryProbe(page);
    const probeKey = Object.getOwnPropertyNames(globalThis)
      .find((key) => key.includes("DEVE_ANDROID_DRAWER_GESTURE_PROBE"));
    assert.ok(probeKey);
    listeners.find(({ type }) => type === "touchstart").callback({
      changedTouches: [{ identifier: 4, clientX: 40, clientY: 100 }],
      touches: [{}],
    });
    listeners.find(({ type }) => type === "touchend").callback({
      changedTouches: [{ identifier: 4, clientX: 120, clientY: 100 }],
      touches: [],
    });
    const signal = listeners[0].signal;
    assert.deepEqual(await takeTouchDeliveryProbe(page), completeLeftDelivery(4));
    assert.equal(signal.aborted, true);
    assert.equal(Object.hasOwn(globalThis, probeKey), false);

    await beginTouchDeliveryProbe(page);
    const failingProbe = globalThis[probeKey];
    const failingSignal = failingProbe.controller.signal;
    failingProbe.events = new Proxy([], {
      get(target, property, receiver) {
        if (property === Symbol.iterator) throw new Error("synthetic read failure");
        return Reflect.get(target, property, receiver);
      },
    });
    await assert.rejects(takeTouchDeliveryProbe(page), /synthetic read failure/);
    assert.equal(failingSignal.aborted, true);
    assert.equal(Object.hasOwn(globalThis, probeKey), false);
  } finally {
    globalThis.window = previousWindow;
  }
});

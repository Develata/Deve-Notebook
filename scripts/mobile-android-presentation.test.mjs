import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import {
  beginTouchDeliveryProbe,
  classifyAndroidDrawerGestureDelivery,
  selectNonInteractiveSwipePoints,
  shouldRetryAndroidDrawerGestureDelivery,
  takeTouchDeliveryProbe,
} from "./lib/android-drawer-touch-proof.mjs";
import {
  drawerVisualStateMatches,
  openDrawerWithObservedNativeSwipe,
  waitForDrawerVisualState,
} from "./lib/android-presentation-proof.mjs";

const read = (path) => fs.readFileSync(new URL(path, import.meta.url), "utf8");
const localJourney = read("./smoke-mobile-android-lifecycle.mjs");
const remoteJourney = read("./smoke-mobile-android-remote-browser.mjs");
const writableEvidence = read("./lib/android-writable-evidence.mjs");
const presentationProof = read("./lib/android-presentation-proof.mjs");
const touchProof = read("./lib/android-drawer-touch-proof.mjs");
const nativeDispatcher = read("../apps/mobile/gen/android/app/src/main/java/dev/deve/notebook/mobile/NativePresentationDispatcher.kt");
const webPresentation = read("../apps/web/src/components/mobile_layout/native_presentation.rs");

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

function createSwipeHarness({ deliveries = [], openFailures = [], adbFailure = false } = {}) {
  const calls = { adb: 0, begin: 0, take: 0, select: 0, visual: [] };
  return {
    calls,
    adbCommand() {
      calls.adb += 1;
      if (adbFailure) throw new Error("synthetic adb failure");
    },
    testing: {
      async selectNonInteractiveSwipePoints() {
        calls.select += 1;
        return [{ yCss: 100, targetTag: "main" }];
      },
      async beginTouchDeliveryProbe() {
        calls.begin += 1;
      },
      async takeTouchDeliveryProbe() {
        const delivery = deliveries[calls.take] ?? [];
        calls.take += 1;
        return delivery;
      },
      async waitForDrawerVisualState(_page, side, open) {
        calls.visual.push({ side, open });
        if (open && openFailures.includes(calls.adb)) {
          throw new Error("synthetic drawer remained closed");
        }
      },
    },
  };
}

async function runLeftSwipe(harness) {
  return openDrawerWithObservedNativeSwipe({}, {
    adbCommand: harness.adbCommand,
    side: "left",
    startPx: 80,
    distancePx: 160,
    density: 2,
    waitUntil: async () => true,
    testing: harness.testing,
  });
}

test("native presentation is re-admitted after same-WebView reload before drawer gestures", () => {
  const localReload = localJourney.indexOf("await reloadWithWebSocketDeliveryGate(page)");
  const localProof = localJourney.indexOf("await proveAndroidDrawerGesturesAfterReload(page");
  const remoteReload = remoteJourney.indexOf("await observeRemoteGeneration(page, observations)");
  const remoteProof = remoteJourney.indexOf("await waitForAcceptedAndroidPresentation(page");
  assert.ok(localReload >= 0 && localReload < localProof);
  assert.ok(remoteReload >= 0 && remoteReload < remoteProof);
  assert.match(presentationProof, /data-deve-native-presentation/);
  assert.match(presentationProof, /data-deve-mobile-drawer=/);
  assert.match(presentationProof, /side: "left"/);
  assert.match(presentationProof, /side: "right"/);
  assert.match(touchProof, /document\.elementFromPoint\(x, y\)/);
  assert.match(touchProof, /target\.closest\(blockingSelector\)/);
  assert.match(presentationProof, /DRAWER_TRANSITION_SETTLE_MS/);
  assert.match(presentationProof, /MAX_SWIPE_DELIVERY_ATTEMPTS = 2/);
  assert.match(presentationProof, /assert\.equal\(pidAfter, pidBefore/);
  assert.match(writableEvidence, /nativeDrawerGesturesAfterReload:/);
  assert.match(remoteJourney, /nativeSystemGestureInsetsAcceptedAfterReload: true/);
});

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
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("missing", 1), true);
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("cancelled", 1), true);
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("complete", 1), false);
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("invalid", 1), false);
  assert.equal(shouldRetryAndroidDrawerGestureDelivery("missing", 2), false);
});

test("drawer hit testing rejects action controls but admits the editor swipe surface", async () => {
  const mobileRoot = {};
  const makePage = (blocked) => ({
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
});

test("drawer visual settlement requires marker, accessibility, hit testing, and geometry", () => {
  const leftOpen = { open: "true", ariaHidden: "false", pointerEvents: "auto", left: 0, right: 320, width: 320, viewportWidth: 400 };
  const leftClosed = { open: "false", ariaHidden: "true", pointerEvents: "none", left: -320, right: 0, width: 320, viewportWidth: 400 };
  const rightOpen = { open: "true", ariaHidden: "false", pointerEvents: "auto", left: 80, right: 400, width: 320, viewportWidth: 400 };
  const rightClosed = { open: "false", ariaHidden: "true", pointerEvents: "none", left: 400, right: 720, width: 320, viewportWidth: 400 };
  assert.equal(drawerVisualStateMatches(leftOpen, "left", true), true);
  assert.equal(drawerVisualStateMatches(leftClosed, "left", false), true);
  assert.equal(drawerVisualStateMatches(rightOpen, "right", true), true);
  assert.equal(drawerVisualStateMatches(rightClosed, "right", false), true);
  assert.equal(drawerVisualStateMatches({ ...leftOpen, ariaHidden: "true" }, "left", true), false);
  assert.equal(drawerVisualStateMatches({ ...leftClosed, right: 40 }, "left", false), false);
  assert.equal(drawerVisualStateMatches({ ...rightOpen, pointerEvents: "none" }, "right", true), false);
});

test("drawer visual settlement rejects a transient open state", async () => {
  const stableOpen = { open: "true", ariaHidden: "false", pointerEvents: "auto", left: 0, right: 320, width: 320, viewportWidth: 400 };
  const closed = { open: "false", ariaHidden: "true", pointerEvents: "none", left: -320, right: 0, width: 320, viewportWidth: 400 };
  const originalNow = Date.now;
  let now = 0;
  Date.now = () => now;
  try {
    const page = {
      calls: 0,
      async call(callback, side) {
        const state = this.calls < 2 ? stableOpen : closed;
        this.calls += 1;
        const previousDocument = globalThis.document;
        const previousGetComputedStyle = globalThis.getComputedStyle;
        const previousWindow = globalThis.window;
        globalThis.window = { innerWidth: state.viewportWidth };
        globalThis.getComputedStyle = () => ({ pointerEvents: state.pointerEvents });
        globalThis.document = {
          querySelector(selector) {
            if (!selector.includes(side)) return null;
            return {
              getAttribute(name) {
                return name === "data-deve-mobile-drawer-open" ? state.open : state.ariaHidden;
              },
              getBoundingClientRect() {
                return { left: state.left, right: state.right, width: state.width };
              },
            };
          },
        };
        try {
          return callback(side);
        } finally {
          globalThis.document = previousDocument;
          globalThis.getComputedStyle = previousGetComputedStyle;
          globalThis.window = previousWindow;
        }
      },
    };
    const waitUntil = async (_label, predicate) => {
      for (let index = 0; index < 6; index += 1) {
        if (await predicate()) return true;
        now += 100;
      }
      throw new Error("synthetic visual timeout");
    };
    await assert.rejects(
      waitForDrawerVisualState(page, "left", true, waitUntil),
      /synthetic visual timeout/,
    );
    page.calls = 0;
    now = 0;
    page.call = async (callback, side) => {
      const previousDocument = globalThis.document;
      const previousGetComputedStyle = globalThis.getComputedStyle;
      const previousWindow = globalThis.window;
      globalThis.window = { innerWidth: stableOpen.viewportWidth };
      globalThis.getComputedStyle = () => ({ pointerEvents: stableOpen.pointerEvents });
      globalThis.document = {
        querySelector: () => ({
          getAttribute: (name) => name === "data-deve-mobile-drawer-open" ? stableOpen.open : stableOpen.ariaHidden,
          getBoundingClientRect: () => ({ left: stableOpen.left, right: stableOpen.right, width: stableOpen.width }),
        }),
      };
      try {
        return callback(side);
      } finally {
        globalThis.document = previousDocument;
        globalThis.getComputedStyle = previousGetComputedStyle;
        globalThis.window = previousWindow;
      }
    };
    await waitForDrawerVisualState(page, "left", true, waitUntil);
  } finally {
    Date.now = originalNow;
  }
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

test("drawer proof requires an initially closed drawer and a complete observed transition", async () => {
  const initialOpen = createSwipeHarness();
  initialOpen.testing.waitForDrawerVisualState = async (_page, _side, open) => {
    if (!open) throw new Error("drawer already open");
  };
  await assert.rejects(runLeftSwipe(initialOpen), /drawer already open/);
  assert.deepEqual(
    { adb: initialOpen.calls.adb, begin: initialOpen.calls.begin, take: initialOpen.calls.take },
    { adb: 0, begin: 0, take: 0 },
  );

  const success = createSwipeHarness({ deliveries: [completeLeftDelivery()] });
  assert.deepEqual(await runLeftSwipe(success), { attempts: 1, targetTag: "main" });
  assert.deepEqual(success.calls.visual, [
    { side: "left", open: false },
    { side: "left", open: true },
  ]);
  assert.deepEqual(
    { adb: success.calls.adb, begin: success.calls.begin, take: success.calls.take },
    { adb: 1, begin: 1, take: 1 },
  );
});

test("drawer proof retries one missing delivery but not complete or invalid delivery", async () => {
  const retry = createSwipeHarness({ deliveries: [[], completeLeftDelivery()], openFailures: [1] });
  assert.deepEqual(await runLeftSwipe(retry), { attempts: 2, targetTag: "main" });
  assert.deepEqual(
    { adb: retry.calls.adb, begin: retry.calls.begin, take: retry.calls.take, select: retry.calls.select },
    { adb: 2, begin: 2, take: 2, select: 2 },
  );
  assert.deepEqual(retry.calls.visual, [
    { side: "left", open: false },
    { side: "left", open: true },
    { side: "left", open: false },
    { side: "left", open: true },
  ]);

  const completeClosed = createSwipeHarness({ deliveries: [completeLeftDelivery()], openFailures: [1] });
  await assert.rejects(runLeftSwipe(completeClosed), /stayed closed after complete WebView touch delivery/);
  assert.equal(completeClosed.calls.adb, 1);

  const invalidClosed = createSwipeHarness({
    deliveries: [[completeLeftDelivery()[0], { ...completeLeftDelivery()[1], identifier: 8 }]],
    openFailures: [1],
  });
  await assert.rejects(runLeftSwipe(invalidClosed), /swipe delivery invalid after bounded retry/);
  assert.equal(invalidClosed.calls.adb, 1);
});

test("drawer proof performs best-effort probe cleanup when ADB fails", async () => {
  const failure = createSwipeHarness({ adbFailure: true });
  await assert.rejects(runLeftSwipe(failure), /drawer ADB swipe command failed/);
  assert.deepEqual(
    { adb: failure.calls.adb, begin: failure.calls.begin, take: failure.calls.take },
    { adb: 1, begin: 1, take: 1 },
  );
});

test("current presentation becomes pending before replacement Insets can re-arm gestures", () => {
  const pending = nativeDispatcher.indexOf("scheduleInvalidation(source, webViewGeneration, epoch, 0)");
  const ready = nativeDispatcher.indexOf("schedulePublish(source, generation, epoch, 0)");
  assert.ok(pending >= 0 && ready >= 0 && pending < ready);
  assert.match(nativeDispatcher, /system-gesture-insets-pending/);
  assert.match(nativeDispatcher, /ViewCompat\.setOnApplyWindowInsetsListener\(observer\)/);
  assert.match(nativeDispatcher, /FrameLayout\.LayoutParams\(0, 0\)/);
  assert.match(webPresentation, /Some\("system-gesture-insets-pending"\)/);
  assert.match(webPresentation, /set_system_gesture_insets\.set\(None\)/);
  assert.match(webPresentation, /if order < apply_order\.get\(\)/);
});

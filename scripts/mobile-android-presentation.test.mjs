import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import {
  androidSafeAreaStateMatches,
  drawerVisualStateIsSemanticallyOpen,
  drawerVisualStateMatches,
  openDrawerWithObservedNativeSwipe,
  parseEditorSelectionIdentity,
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

const leftOpen = {
  open: "true", ariaHidden: "false", pointerEvents: "auto",
  left: 0, right: 320, width: 320, viewportWidth: 400,
  safeTopCss: 35, closeControlTop: 39,
};
const leftClosed = {
  open: "false", ariaHidden: "true", pointerEvents: "none",
  left: -320, right: 0, width: 320, viewportWidth: 400,
};

function createSwipeHarness({
  deliveries = [], openFailures = [], adbFailure = false, focusFailure = false,
  initialState = leftClosed,
} = {}) {
  const calls = {
    adb: 0, adbArgs: [], begin: 0, focus: 0, take: 0, select: 0, visual: [], order: [],
  };
  return {
    calls,
    adbCommand(...args) {
      calls.order.push("adb");
      calls.adb += 1;
      calls.adbArgs.push(args);
      if (adbFailure) throw new Error("synthetic adb failure");
    },
    testing: {
      async readDrawerVisualState() {
        return initialState;
      },
      async waitForCurrentWebViewInputFocus() {
        calls.order.push("focus");
        calls.focus += 1;
        if (focusFailure) throw new Error("synthetic focus timeout");
      },
      async selectNonInteractiveSwipePoints() {
        calls.order.push("select");
        calls.select += 1;
        return [{ yCss: 100, targetTag: "main" }];
      },
      async beginTouchDeliveryProbe() {
        calls.order.push("begin");
        calls.begin += 1;
      },
      async takeTouchDeliveryProbe() {
        calls.order.push("take");
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
  assert.match(presentationProof, /data-deve-native-safe-area/);
  assert.match(presentationProof, /safeTopPx/);
  assert.match(presentationProof, /safeBottomPx/);
  assert.match(presentationProof, /data-deve-mobile-drawer=/);
  assert.match(presentationProof, /side: "left"/);
  assert.match(presentationProof, /side: "right"/);
  assert.match(localJourney, /proveAndroidWorkEditDrawerGestures/);
  assert.match(presentationProof, /data-deve-mobile-work-edit-swipe-surface/);
  assert.match(presentationProof, /requiredClosestSelector/);
  assert.match(presentationProof, /workEditSelectionStable: true/);
  assert.match(touchProof, /document\.elementFromPoint\(x, y\)/);
  assert.match(touchProof, /target\.closest\(blockingSelector\)/);
  assert.match(presentationProof, /DRAWER_TRANSITION_SETTLE_MS/);
  assert.match(
    presentationProof,
    /MAX_SWIPE_DELIVERY_ATTEMPTS = SWIPE_Y_FRACTIONS\.length/,
  );
  assert.match(presentationProof, /assert\.equal\(pidAfter, pidBefore/);
  assert.match(writableEvidence, /nativeDrawerGesturesAfterReload:/);
  assert.match(writableEvidence, /drawerGestureProof\.workEditSelectionStable/);
  assert.match(remoteJourney, /nativeSystemGestureInsetsAcceptedAfterReload: true/);
});

test("native presentation requires a settled system-bar safe area around mobile chrome", () => {
  const valid = {
    presentation: {
      kind: "system-gesture-insets",
      generation: 2,
      epoch: 9,
      widthPx: 1080,
      heightPx: 2400,
      leftPx: 62,
      rightPx: 62,
      safeTopPx: 94,
      safeBottomPx: 68,
      density: 2.75,
    },
    accepted: true,
    safeAreaReady: true,
    safeTopCss: 35,
    safeBottomCss: 25,
    viewportWidth: 1080 / 2.75,
    viewportHeight: 2400 / 2.75,
    headerTop: 0,
    headerControlTop: 36,
    footerBottom: 2400 / 2.75,
    footerPaddingBottom: 25,
    bottomControlBottom: (2400 / 2.75) - 26,
    headerBackground: "rgb(239, 236, 227)",
    footerBackground: "rgb(239, 236, 227)",
  };
  assert.equal(androidSafeAreaStateMatches(valid), true);
  assert.equal(androidSafeAreaStateMatches({ ...valid, safeAreaReady: false }), false);
  assert.equal(androidSafeAreaStateMatches({ ...valid, safeTopCss: 0 }), false);
  assert.equal(androidSafeAreaStateMatches({ ...valid, headerTop: 35 }), false);
  assert.equal(androidSafeAreaStateMatches({ ...valid, headerControlTop: 20 }), false);
  assert.equal(androidSafeAreaStateMatches({ ...valid, footerPaddingBottom: 0 }), false);
  assert.equal(androidSafeAreaStateMatches({
    ...valid,
    bottomControlBottom: valid.viewportHeight - valid.safeBottomCss + 8,
  }), false);
  assert.equal(androidSafeAreaStateMatches({
    ...valid,
    presentation: { ...valid.presentation, safeTopPx: 2350, safeBottomPx: 68 },
  }), false);
});

test("Work Edit selection proof rejects null, malformed, and invalid identities", () => {
  assert.deepEqual(
    parseEditorSelectionIdentity('{"from":3,"to":7,"rangeCount":1}'),
    { from: 3, to: 7, rangeCount: 1 },
  );
  for (const invalid of [
    null,
    "null",
    "not-json",
    '{"from":-1,"to":0,"rangeCount":1}',
    '{"from":7,"to":3,"rangeCount":1}',
    '{"from":0,"to":0,"rangeCount":0}',
    '{"from":0.5,"to":1,"rangeCount":1}',
  ]) {
    assert.equal(parseEditorSelectionIdentity(invalid), null);
  }
});

test("drawer visual settlement requires marker, accessibility, hit testing, and geometry", () => {
  const rightOpen = { open: "true", ariaHidden: "false", pointerEvents: "auto", left: 80, right: 400, width: 320, viewportWidth: 400, safeTopCss: 35, closeControlTop: 39 };
  const rightClosed = { open: "false", ariaHidden: "true", pointerEvents: "none", left: 400, right: 720, width: 320, viewportWidth: 400 };
  assert.equal(drawerVisualStateMatches(leftOpen, "left", true), true);
  assert.equal(drawerVisualStateMatches(leftClosed, "left", false), true);
  assert.equal(drawerVisualStateMatches(rightOpen, "right", true), true);
  assert.equal(drawerVisualStateMatches(rightClosed, "right", false), true);
  assert.equal(drawerVisualStateMatches({ ...leftOpen, ariaHidden: "true" }, "left", true), false);
  assert.equal(drawerVisualStateMatches({ ...leftClosed, right: 40 }, "left", false), false);
  assert.equal(drawerVisualStateMatches({ ...rightOpen, pointerEvents: "none" }, "right", true), false);
  assert.equal(drawerVisualStateMatches({ ...leftOpen, closeControlTop: 20 }, "left", true), false);
  assert.equal(drawerVisualStateIsSemanticallyOpen(leftOpen), true);
  assert.equal(drawerVisualStateIsSemanticallyOpen(leftClosed), false);
  assert.equal(drawerVisualStateIsSemanticallyOpen({ ...leftOpen, pointerEvents: "none" }), false);
  assert.equal(drawerVisualStateIsSemanticallyOpen({ ...leftOpen, pointerEvents: undefined }), false);
});

test("drawer settlement failure redacts the underlying observation error", async () => {
  const page = {
    call: () => Promise.reject(new Error("secret=/private/runner/path")),
  };
  const waitUntil = async (_label, predicate) => predicate();
  await assert.rejects(waitForDrawerVisualState(page, "left", false, waitUntil), (error) => {
    assert.equal(error.message, "left drawer closed visual settlement failed; last=null");
    assert.doesNotMatch(error.message, /secret|private|runner|path/);
    return true;
  });
});

test("drawer visual settlement rejects a transient open state", async () => {
  const stableOpen = { open: "true", ariaHidden: "false", pointerEvents: "auto", left: 0, right: 320, width: 320, viewportWidth: 400, safeTopCss: 35, closeControlTop: 39 };
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
        const header = {};
        globalThis.getComputedStyle = (node) => node === header
          ? { paddingTop: `${state.safeTopCss ?? 0}px` }
          : { pointerEvents: state.pointerEvents };
        globalThis.document = {
          querySelector(selector) {
            if (selector.includes("data-deve-mobile-header")) return header;
            if (!selector.includes(side)) return null;
            return {
              getAttribute(name) {
                return name === "data-deve-mobile-drawer-open" ? state.open : state.ariaHidden;
              },
              getBoundingClientRect() {
                return { left: state.left, right: state.right, width: state.width };
              },
              querySelector() {
                return state.closeControlTop == null ? null : {
                  getBoundingClientRect: () => ({ top: state.closeControlTop }),
                };
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
    await assert.rejects(waitForDrawerVisualState(page, "left", true, waitUntil), (error) => {
      assert.match(error.message, /left drawer open visual settlement failed/);
      assert.match(error.message, /"open":"false"/);
      assert.doesNotMatch(error.message, /synthetic visual timeout/);
      return true;
    });
    page.calls = 0;
    now = 0;
    page.call = async (callback, side) => {
      const previousDocument = globalThis.document;
      const previousGetComputedStyle = globalThis.getComputedStyle;
      const previousWindow = globalThis.window;
      globalThis.window = { innerWidth: stableOpen.viewportWidth };
      const header = {};
      globalThis.getComputedStyle = (node) => node === header
        ? { paddingTop: `${stableOpen.safeTopCss}px` }
        : { pointerEvents: stableOpen.pointerEvents };
      globalThis.document = {
        querySelector: (selector) => selector.includes("data-deve-mobile-header") ? header : ({
          getAttribute: (name) => name === "data-deve-mobile-drawer-open" ? stableOpen.open : stableOpen.ariaHidden,
          getBoundingClientRect: () => ({ left: stableOpen.left, right: stableOpen.right, width: stableOpen.width }),
          querySelector: () => ({
            getBoundingClientRect: () => ({ top: stableOpen.closeControlTop }),
          }),
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

test("drawer proof normalizes only a semantically open initial drawer before contact", async () => {
  const initialOpen = createSwipeHarness({
    initialState: leftOpen,
    deliveries: [completeLeftDelivery()],
  });
  assert.deepEqual(await runLeftSwipe(initialOpen), { attempts: 1, targetTag: "main" });
  assert.deepEqual(initialOpen.calls.adbArgs, [
    ["shell", "input", "keyevent", "4"],
    ["shell", "input", "swipe", "80", "200", "240", "200", "350"],
  ]);

  const inconsistentClosed = createSwipeHarness({
    initialState: { ...leftClosed, right: 40 },
  });
  inconsistentClosed.testing.waitForDrawerVisualState = async () => {
    throw new Error("drawer closed geometry invalid");
  };
  await assert.rejects(runLeftSwipe(inconsistentClosed), /drawer closed geometry invalid/);
  assert.deepEqual(
    {
      adb: inconsistentClosed.calls.adb,
      begin: inconsistentClosed.calls.begin,
      take: inconsistentClosed.calls.take,
    },
    { adb: 0, begin: 0, take: 0 },
  );

  const unfocused = createSwipeHarness({ focusFailure: true });
  await assert.rejects(runLeftSwipe(unfocused), /synthetic focus timeout/);
  assert.deepEqual(unfocused.calls.order, ["focus"]);

  const success = createSwipeHarness({ deliveries: [completeLeftDelivery()] });
  assert.deepEqual(await runLeftSwipe(success), { attempts: 1, targetTag: "main" });
  assert.deepEqual(success.calls.visual, [
    { side: "left", open: false },
    { side: "left", open: true },
  ]);
  assert.deepEqual(
    { adb: success.calls.adb, begin: success.calls.begin, focus: success.calls.focus, take: success.calls.take },
    { adb: 1, begin: 1, focus: 1, take: 1 },
  );
  assert.deepEqual(success.calls.order, ["focus", "select", "begin", "adb", "take"]);
});

test("drawer proof retries two missing deliveries but not complete or invalid delivery", async () => {
  const retry = createSwipeHarness({
    deliveries: [[], [], completeLeftDelivery()],
    openFailures: [1, 2],
  });
  assert.deepEqual(await runLeftSwipe(retry), { attempts: 3, targetTag: "main" });
  assert.deepEqual(
    { adb: retry.calls.adb, begin: retry.calls.begin, focus: retry.calls.focus, take: retry.calls.take, select: retry.calls.select },
    { adb: 3, begin: 3, focus: 3, take: 3, select: 3 },
  );
  assert.deepEqual(retry.calls.visual, [
    { side: "left", open: false },
    { side: "left", open: true },
    { side: "left", open: false },
    { side: "left", open: true },
    { side: "left", open: false },
    { side: "left", open: true },
  ]);
  assert.deepEqual(retry.calls.order, [
    "focus", "select", "begin", "adb", "take",
    "focus", "select", "begin", "adb", "take",
    "focus", "select", "begin", "adb", "take",
  ]);

  const exhausted = createSwipeHarness({
    deliveries: [[], [], []],
    openFailures: [1, 2, 3],
  });
  await assert.rejects(runLeftSwipe(exhausted), /missing after 3 bounded attempts/);
  assert.equal(exhausted.calls.adb, 3);

  const completeClosed = createSwipeHarness({ deliveries: [completeLeftDelivery()], openFailures: [1] });
  await assert.rejects(runLeftSwipe(completeClosed), /stayed closed after complete WebView touch delivery/);
  assert.equal(completeClosed.calls.adb, 1);

  const invalidClosed = createSwipeHarness({
    deliveries: [[completeLeftDelivery()[0], { ...completeLeftDelivery()[1], identifier: 8 }]],
    openFailures: [1],
  });
  await assert.rejects(runLeftSwipe(invalidClosed), /swipe delivery invalid after 1 bounded attempt/);
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

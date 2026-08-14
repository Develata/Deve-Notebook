import assert from "node:assert/strict";
import {
  beginTouchDeliveryProbe,
  classifyAndroidDrawerGestureDelivery,
  selectNonInteractiveSwipePoints,
  shouldRetryAndroidDrawerGestureDelivery,
  takeTouchDeliveryProbe,
} from "./android-drawer-touch-proof.mjs";

const SAFE_FLOOR_CSS = 24;
const ACTIVATION_OFFSET_CSS = 18;
const SWIPE_DISTANCE_CSS = 80;
const DRAWER_TRANSITION_SETTLE_MS = 250;
const SWIPE_DELIVERY_TIMEOUT_MS = 4000;
const MAX_SWIPE_DELIVERY_ATTEMPTS = 2;
const SWIPE_Y_FRACTIONS = [0.35, 0.58, 0.76];

export function drawerVisualStateMatches(state, side, open) {
  if (!state || !["left", "right"].includes(side)) return false;
  const geometryReady = side === "left"
    ? (open ? Math.abs(state.left) <= 1 : state.right <= 1)
    : (open ? Math.abs(state.right - state.viewportWidth) <= 1
      : state.left >= state.viewportWidth - 1);
  return state.open === String(open)
    && state.ariaHidden === String(!open)
    && (open ? state.pointerEvents !== "none" : state.pointerEvents === "none")
    && state.width > 0
    && geometryReady
    && (!open || (
      Number.isFinite(state.safeTopCss)
      && Number.isFinite(state.closeControlTop)
      && state.closeControlTop + 0.5 >= state.safeTopCss
    ));
}

export function androidSafeAreaStateMatches(state) {
  const value = state?.presentation;
  if (!state?.accepted
    || !state?.safeAreaReady
    || value?.kind !== "system-gesture-insets"
    || !Number.isSafeInteger(value.generation)
    || value.generation <= 0
    || !Number.isSafeInteger(value.epoch)
    || value.epoch <= 0
    || !Number.isFinite(value.widthPx)
    || !Number.isFinite(value.heightPx)
    || !Number.isFinite(value.leftPx)
    || !Number.isFinite(value.rightPx)
    || !Number.isFinite(value.safeTopPx)
    || !Number.isFinite(value.safeBottomPx)
    || !Number.isFinite(value.density)
    || value.widthPx <= 0
    || value.heightPx <= 0
    || value.leftPx < 0
    || value.rightPx < 0
    || value.safeTopPx < 0
    || value.safeBottomPx < 0
    || value.safeTopPx + value.safeBottomPx > value.heightPx
    || value.density <= 0
    || !Number.isFinite(state.viewportWidth)
    || !Number.isFinite(state.viewportHeight)
    || Math.abs((value.widthPx / value.density) - state.viewportWidth) > 2) return false;
  const expectedTopCss = Math.ceil(value.safeTopPx / value.density);
  const expectedBottomCss = Math.ceil(value.safeBottomPx / value.density);
  return Number.isFinite(state.safeTopCss)
    && Number.isFinite(state.safeBottomCss)
    && state.safeTopCss + 0.5 >= expectedTopCss
    && state.safeBottomCss + 0.5 >= expectedBottomCss
    && Number.isFinite(state.headerTop)
    && Math.abs(state.headerTop) <= 1
    && Number.isFinite(state.headerControlTop)
    && state.headerControlTop + 0.5 >= state.safeTopCss
    && Number.isFinite(state.footerBottom)
    && Math.abs(state.footerBottom - state.viewportHeight) <= 1
    && Number.isFinite(state.footerPaddingBottom)
    && state.footerPaddingBottom + 0.5 >= state.safeBottomCss
    && Number.isFinite(state.bottomControlBottom)
    && state.bottomControlBottom <= state.viewportHeight - state.safeBottomCss + 1
    && typeof state.headerBackground === "string"
    && state.headerBackground !== "rgba(0, 0, 0, 0)"
    && state.headerBackground === state.footerBackground;
}

export async function waitForAcceptedAndroidPresentation(page, waitUntil, timeout = 10000) {
  const state = await waitUntil(
    "Web-accepted generation-bound Android presentation and safe area",
    async () => {
      const state = await page.call(() => {
      const value = window.__DEVE_ANDROID_PRESENTATION__;
      const accepted = document.querySelector('[data-deve-layout-mode="mobile"]')
        ?.getAttribute("data-deve-native-presentation") === "ready";
      const root = document.documentElement;
      const probe = document.createElement("div");
      probe.style.cssText = "position:fixed;visibility:hidden;padding-top:var(--deve-safe-area-top);padding-bottom:var(--deve-safe-area-bottom)";
      document.body.appendChild(probe);
      const probeStyle = getComputedStyle(probe);
      const safeTopCss = Number.parseFloat(probeStyle.paddingTop);
      const safeBottomCss = Number.parseFloat(probeStyle.paddingBottom);
      probe.remove();
      const header = document.querySelector('[data-deve-mobile-header="topbar"]');
      const headerControl = header?.querySelector('[data-deve-mobile-touch-target="topbar_buttons"]');
      const footer = document.querySelector("[data-deve-mobile-bottom-bar]");
      const bottomControl = footer?.querySelector('[data-deve-mobile-touch-target="bottom_bar_toggle"]');
      const headerStyle = header ? getComputedStyle(header) : null;
      const footerStyle = footer ? getComputedStyle(footer) : null;
      return {
        presentation: value ? { ...value } : null,
        accepted,
        safeAreaReady: root.getAttribute("data-deve-native-safe-area") === "ready",
        safeTopCss,
        safeBottomCss,
        viewportWidth: window.innerWidth,
        viewportHeight: window.innerHeight,
        headerTop: header?.getBoundingClientRect().top ?? null,
        headerControlTop: headerControl?.getBoundingClientRect().top ?? null,
        footerBottom: footer?.getBoundingClientRect().bottom ?? null,
        footerPaddingBottom: footerStyle ? Number.parseFloat(footerStyle.paddingBottom) : null,
        bottomControlBottom: bottomControl?.getBoundingClientRect().bottom ?? null,
        headerBackground: headerStyle?.backgroundColor ?? null,
        footerBackground: footerStyle?.backgroundColor ?? null,
      };
    });
      return androidSafeAreaStateMatches(state) ? state : null;
    },
    timeout,
  );
  return {
    ...state.presentation,
    safeAreaTopCss: state.safeTopCss,
    safeAreaBottomCss: state.safeBottomCss,
    viewportHeightCss: state.viewportHeight,
  };
}

function readDrawerVisualState(page, side) {
  return page.call((drawerSide) => {
    const drawer = document.querySelector(`[data-deve-mobile-drawer="${drawerSide}"]`);
    if (!drawer) return null;
    const rect = drawer.getBoundingClientRect();
    const closeControl = drawer.querySelector('[data-deve-mobile-touch-target="drawer_close_buttons"]');
    const mobileHeader = document.querySelector('[data-deve-mobile-header="topbar"]');
    const safeTopCss = mobileHeader
      ? Number.parseFloat(getComputedStyle(mobileHeader).paddingTop)
      : Number.NaN;
    return {
      open: drawer.getAttribute("data-deve-mobile-drawer-open"),
      ariaHidden: drawer.getAttribute("aria-hidden"),
      pointerEvents: getComputedStyle(drawer).pointerEvents,
      left: rect.left,
      right: rect.right,
      width: rect.width,
      viewportWidth: window.innerWidth,
      safeTopCss,
      closeControlTop: closeControl?.getBoundingClientRect().top ?? null,
    };
  }, side);
}

export async function waitForDrawerVisualState(page, side, open, waitUntil, timeout = 5000) {
  let matchedSince = null;
  await waitUntil(`${side} drawer ${open ? "open" : "closed"} visual settlement`, async () => {
    const state = await readDrawerVisualState(page, side);
    if (!drawerVisualStateMatches(state, side, open)) {
      matchedSince = null;
      return false;
    }
    matchedSince ??= Date.now();
    return Date.now() - matchedSince >= DRAWER_TRANSITION_SETTLE_MS;
  }, timeout);
}

export async function openDrawerWithObservedNativeSwipe(page, {
  adbCommand,
  side,
  startPx,
  distancePx,
  density,
  waitUntil,
  requiredClosestSelector = null,
  testing = {},
}) {
  const selectPoints = testing.selectNonInteractiveSwipePoints ?? selectNonInteractiveSwipePoints;
  const beginProbe = testing.beginTouchDeliveryProbe ?? beginTouchDeliveryProbe;
  const takeProbe = testing.takeTouchDeliveryProbe ?? takeTouchDeliveryProbe;
  const waitForVisualState = testing.waitForDrawerVisualState ?? waitForDrawerVisualState;
  const direction = side === "left" ? 1 : -1;
  await waitForVisualState(page, side, false, waitUntil);
  let lastDelivery = "missing";
  let lastEvents = [];

  for (let attempt = 0; attempt < MAX_SWIPE_DELIVERY_ATTEMPTS; attempt += 1) {
    const points = await selectPoints(
      page, startPx / density, SWIPE_Y_FRACTIONS, requiredClosestSelector,
    );
    assert.ok(points.length > 0, `${side} activation band has no non-interactive hit-tested point`);
    const point = points[Math.min(attempt, points.length - 1)];
    const yPx = Math.round(point.yCss * density);
    const expectedDelivery = {
      startXCss: startPx / density,
      startYCss: point.yCss,
      endXCss: (startPx + direction * distancePx) / density,
      endYCss: point.yCss,
      direction,
    };
    await beginProbe(page);
    try {
      adbCommand(
        "shell", "input", "swipe",
        String(startPx), String(yPx), String(startPx + direction * distancePx), String(yPx), "350",
      );
    } catch (error) {
      await takeProbe(page).catch(() => {});
      throw new Error(`${side} drawer ADB swipe command failed`, { cause: error });
    }
    try {
      await waitForVisualState(
        page, side, true, waitUntil, SWIPE_DELIVERY_TIMEOUT_MS,
      );
    } catch (error) {
      try {
        lastEvents = await takeProbe(page);
      } catch (_probeError) {
        throw error;
      }
      lastDelivery = classifyAndroidDrawerGestureDelivery(lastEvents, expectedDelivery);
      if (lastDelivery === "complete") {
        throw new Error(`${side} drawer stayed closed after complete WebView touch delivery: ${JSON.stringify(lastEvents)}`, { cause: error });
      }
      if (!shouldRetryAndroidDrawerGestureDelivery(lastDelivery, attempt + 1)) {
        throw new Error(`${side} drawer swipe delivery ${lastDelivery} after bounded retry: ${JSON.stringify(lastEvents)}`, { cause: error });
      }
      await waitForVisualState(page, side, false, waitUntil);
      continue;
    }
    lastEvents = await takeProbe(page);
    lastDelivery = classifyAndroidDrawerGestureDelivery(lastEvents, expectedDelivery);
    assert.equal(lastDelivery, "complete", `${side} drawer opened without a complete observed WebView touch`);
    return { attempts: attempt + 1, targetTag: point.targetTag };
  }
  throw new Error(`${side} drawer swipe ended unexpectedly`);
}

export function parseEditorSelectionIdentity(value) {
  if (typeof value !== "string") return null;
  try {
    const parsed = JSON.parse(value);
    if (!parsed
      || !Number.isSafeInteger(parsed.from)
      || !Number.isSafeInteger(parsed.to)
      || !Number.isSafeInteger(parsed.rangeCount)
      || parsed.from < 0
      || parsed.to < parsed.from
      || parsed.rangeCount < 1) return null;
    return { from: parsed.from, to: parsed.to, rangeCount: parsed.rangeCount };
  } catch (_error) {
    return null;
  }
}

async function readWorkEditSwipeState(page) {
  const raw = await page.call(() => {
    const surface = document.querySelector(
      '[data-deve-mobile-work-edit-swipe-surface="true"]',
    );
    const selectionIdentity = globalThis.__deveWebBridge
      ?.get?.("getEditorSelectionIdentity")?.() ?? null;
    return surface ? selectionIdentity : null;
  });
  const selectionIdentity = parseEditorSelectionIdentity(raw);
  return selectionIdentity ? { selectionIdentity } : null;
}

export async function proveAndroidWorkEditDrawerGestures(page, {
  adbCommand,
  adbOutput,
  appId,
  waitUntil,
}) {
  const presentation = await waitForAcceptedAndroidPresentation(page, waitUntil);
  const before = await waitUntil(
    "Work Edit swipe surface and editor selection",
    () => readWorkEditSwipeState(page),
  );
  const pidBefore = adbOutput("shell", "pidof", appId).trim();
  assert.match(pidBefore, /^[1-9][0-9]*$/, "Work Edit drawer proof requires one stable app PID");
  const startPx = Math.round(presentation.widthPx / 2);
  const distancePx = Math.round(SWIPE_DISTANCE_CSS * presentation.density);
  const requiredClosestSelector =
    '[data-deve-mobile-work-edit-swipe-surface="true"] .cm-content';

  const left = await openDrawerWithObservedNativeSwipe(page, {
    adbCommand, side: "left", startPx, distancePx, density: presentation.density,
    waitUntil, requiredClosestSelector,
  });
  adbCommand("shell", "input", "keyevent", "4");
  await waitForDrawerVisualState(page, "left", false, waitUntil);

  const right = await openDrawerWithObservedNativeSwipe(page, {
    adbCommand, side: "right", startPx, distancePx, density: presentation.density,
    waitUntil, requiredClosestSelector,
  });
  adbCommand("shell", "input", "keyevent", "4");
  await waitForDrawerVisualState(page, "right", false, waitUntil);

  const after = await readWorkEditSwipeState(page);
  assert.ok(after, "Work Edit swipe surface must remain mounted after drawer gestures");
  assert.deepEqual(
    after.selectionIdentity,
    before.selectionIdentity,
    "Work Edit drawer gestures must preserve the editor selection",
  );
  assert.equal(
    adbOutput("shell", "pidof", appId).trim(),
    pidBefore,
    "Work Edit drawer gestures must keep the app PID stable",
  );
  return {
    workEditCenterLeftOpened: true,
    workEditCenterRightOpened: true,
    workEditSelectionStable: true,
    workEditDeliveryAttempts: { left: left.attempts, right: right.attempts },
  };
}

export async function proveAndroidDrawerGesturesAfterReload(page, {
  adbCommand,
  adbOutput,
  appId,
  waitUntil,
}) {
  const presentation = await waitForAcceptedAndroidPresentation(page, waitUntil);
  const pidBefore = adbOutput("shell", "pidof", appId).trim();
  assert.match(pidBefore, /^[1-9][0-9]*$/, "drawer proof requires one stable app PID");

  const safeLeftCss = Math.max(Math.ceil(presentation.leftPx / presentation.density), SAFE_FLOOR_CSS);
  const safeRightCss = Math.max(Math.ceil(presentation.rightPx / presentation.density), SAFE_FLOOR_CSS);
  const leftStartPx = Math.round((safeLeftCss + ACTIVATION_OFFSET_CSS) * presentation.density);
  const rightStartPx = presentation.widthPx
    - Math.round((safeRightCss + ACTIVATION_OFFSET_CSS) * presentation.density);
  const distancePx = Math.round(SWIPE_DISTANCE_CSS * presentation.density);

  const left = await openDrawerWithObservedNativeSwipe(page, {
    adbCommand, side: "left", startPx: leftStartPx, distancePx,
    density: presentation.density, waitUntil,
  });
  adbCommand("shell", "input", "keyevent", "4");
  await waitForDrawerVisualState(page, "left", false, waitUntil);

  const right = await openDrawerWithObservedNativeSwipe(page, {
    adbCommand, side: "right", startPx: rightStartPx, distancePx,
    density: presentation.density, waitUntil,
  });
  adbCommand("shell", "input", "keyevent", "4");
  await waitForDrawerVisualState(page, "right", false, waitUntil);

  const pidAfter = adbOutput("shell", "pidof", appId).trim();
  assert.equal(pidAfter, pidBefore, "native drawer gestures must keep the app PID stable");
  return {
    presentation,
    pidStable: true,
    leftDrawerOpened: true,
    rightDrawerOpened: true,
    deliveryAttempts: { left: left.attempts, right: right.attempts },
  };
}

import assert from "node:assert/strict";

const SAFE_FLOOR_CSS = 24;
const ACTIVATION_OFFSET_CSS = 10;
const SWIPE_DISTANCE_CSS = 80;

export async function waitForAcceptedAndroidPresentation(page, waitUntil, timeout = 10000) {
  return waitUntil(
    "Web-accepted generation-bound Android system gesture insets",
    () => page.call(() => {
      const value = window.__DEVE_ANDROID_PRESENTATION__;
      const accepted = document.querySelector('[data-deve-layout-mode="mobile"]')
        ?.getAttribute("data-deve-native-presentation") === "ready";
      if (!accepted
        || value?.kind !== "system-gesture-insets"
        || !Number.isSafeInteger(value.generation)
        || value.generation <= 0
        || !Number.isSafeInteger(value.epoch)
        || value.epoch <= 0
        || !Number.isFinite(value.widthPx)
        || !Number.isFinite(value.leftPx)
        || !Number.isFinite(value.rightPx)
        || !Number.isFinite(value.density)
        || value.widthPx <= 0
        || value.leftPx < 0
        || value.rightPx < 0
        || value.density <= 0) return null;
      const projectedWidth = value.widthPx / value.density;
      if (Math.abs(projectedWidth - window.innerWidth) > 2) return null;
      return { ...value, viewportHeightCss: window.innerHeight };
    }),
    timeout,
  );
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

  const safeLeftCss = Math.max(
    Math.ceil(presentation.leftPx / presentation.density),
    SAFE_FLOOR_CSS,
  );
  const safeRightCss = Math.max(
    Math.ceil(presentation.rightPx / presentation.density),
    SAFE_FLOOR_CSS,
  );
  const leftStartPx = Math.round(
    (safeLeftCss + ACTIVATION_OFFSET_CSS) * presentation.density,
  );
  const rightStartPx = presentation.widthPx - Math.round(
    (safeRightCss + ACTIVATION_OFFSET_CSS) * presentation.density,
  );
  const distancePx = Math.round(SWIPE_DISTANCE_CSS * presentation.density);
  const viewportHeightPx = Math.round(
    presentation.viewportHeightCss * presentation.density,
  );
  const y = Math.max(200, Math.round(viewportHeightPx * 0.6));

  adbCommand(
    "shell", "input", "swipe",
    String(leftStartPx), String(y), String(leftStartPx + distancePx), String(y), "350",
  );
  await waitUntil("left drawer after native activation-band swipe", () => page.call(() =>
    document.querySelector('[data-deve-mobile-drawer="left"]')
      ?.getAttribute("data-deve-mobile-drawer-open") === "true"));
  adbCommand("shell", "input", "keyevent", "4");
  await waitUntil("left drawer closed by UI Back", () => page.call(() =>
    document.querySelector('[data-deve-mobile-drawer="left"]')
      ?.getAttribute("data-deve-mobile-drawer-open") === "false"));

  adbCommand(
    "shell", "input", "swipe",
    String(rightStartPx), String(y), String(rightStartPx - distancePx), String(y), "350",
  );
  await waitUntil("right drawer after native activation-band swipe", () => page.call(() =>
    document.querySelector('[data-deve-mobile-drawer="right"]')
      ?.getAttribute("data-deve-mobile-drawer-open") === "true"));
  adbCommand("shell", "input", "keyevent", "4");
  await waitUntil("right drawer closed by UI Back", () => page.call(() =>
    document.querySelector('[data-deve-mobile-drawer="right"]')
      ?.getAttribute("data-deve-mobile-drawer-open") === "false"));

  const pidAfter = adbOutput("shell", "pidof", appId).trim();
  assert.equal(pidAfter, pidBefore, "native drawer gestures must keep the app PID stable");
  return { presentation, pidStable: true, leftDrawerOpened: true, rightDrawerOpened: true };
}

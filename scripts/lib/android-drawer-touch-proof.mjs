const TOUCH_COORDINATE_TOLERANCE_CSS = 8;
const BLOCKING_SELECTOR =
  "button, a, input, textarea, select, summary, [role='button'], [data-no-edge-swipe]";
const PROBE_KEY = "__DEVE_ANDROID_DRAWER_GESTURE_PROBE__";

export function classifyAndroidDrawerGestureDelivery(events, expected) {
  if (!Array.isArray(events) || events.length === 0) return "missing";
  if (!Number.isFinite(expected?.startXCss)
    || !Number.isFinite(expected?.startYCss)
    || !Number.isFinite(expected?.endXCss)
    || !Number.isFinite(expected?.endYCss)
    || ![-1, 1].includes(expected?.direction)) return "invalid";
  if (events.length !== 2 || events[0]?.type !== "touchstart") return "invalid";
  const [start, terminal] = events;
  if (!Number.isSafeInteger(start.identifier)
    || start.touchCount !== 1
    || Math.abs(start.x - expected.startXCss) > TOUCH_COORDINATE_TOLERANCE_CSS
    || Math.abs(start.y - expected.startYCss) > TOUCH_COORDINATE_TOLERANCE_CSS
    || terminal.identifier !== start.identifier) return "invalid";
  if (terminal.type === "touchcancel" && terminal.touchCount === 0) return "cancelled";
  if (terminal.type !== "touchend"
    || terminal.touchCount !== 0
    || Math.abs(terminal.x - expected.endXCss) > TOUCH_COORDINATE_TOLERANCE_CSS
    || Math.abs(terminal.y - expected.endYCss) > TOUCH_COORDINATE_TOLERANCE_CSS) {
    return "invalid";
  }
  const dx = terminal.x - start.x;
  const dy = terminal.y - start.y;
  return dx * expected.direction > 0 && Math.abs(dx) > Math.abs(dy) ? "complete" : "invalid";
}

export function shouldRetryAndroidDrawerGestureDelivery(delivery, completedAttempts, maximum) {
  return ["missing", "cancelled"].includes(delivery)
    && Number.isSafeInteger(completedAttempts)
    && Number.isSafeInteger(maximum)
    && completedAttempts > 0
    && maximum > 0
    && completedAttempts < maximum;
}

export async function selectNonInteractiveSwipePoints(
  page, xCss, fractions, requiredClosestSelector = null,
) {
  return page.call((x, candidates, blockingSelector, requiredSelector) => candidates.flatMap((fraction) => {
    const y = Math.round(window.innerHeight * fraction);
    const target = document.elementFromPoint(x, y);
    const mobileRoot = target?.closest('[data-deve-layout-mode="mobile"]');
    const requiredRoot = requiredSelector ? target?.closest(requiredSelector) : mobileRoot;
    if (!target || !mobileRoot || !requiredRoot || target.closest(blockingSelector)) return [];
    return [{ yCss: y, targetTag: target.tagName.toLowerCase() }];
  }), xCss, fractions, BLOCKING_SELECTOR, requiredClosestSelector);
}

export async function beginTouchDeliveryProbe(page) {
  await page.call((key) => {
    globalThis[key]?.controller?.abort();
    const controller = new AbortController();
    const events = [];
    globalThis[key] = { controller, events };
    for (const type of ["touchstart", "touchend", "touchcancel"]) {
      window.addEventListener(type, (event) => {
        const touch = event.changedTouches[0];
        events.push({
          type,
          identifier: Number.isSafeInteger(touch?.identifier) ? touch.identifier : null,
          x: Number.isFinite(touch?.clientX) ? Math.round(touch.clientX) : null,
          y: Number.isFinite(touch?.clientY) ? Math.round(touch.clientY) : null,
          touchCount: event.touches.length,
        });
      }, { capture: true, signal: controller.signal });
    }
  }, PROBE_KEY);
}

export async function takeTouchDeliveryProbe(page) {
  return page.call((key) => {
    const probe = globalThis[key];
    try {
      return Array.isArray(probe?.events) ? [...probe.events] : [];
    } finally {
      probe?.controller?.abort();
      delete globalThis[key];
    }
  }, PROBE_KEY);
}

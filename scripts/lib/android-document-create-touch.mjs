import { tapWebViewPoint } from "./android-webview-pointer.mjs";
import {
  armExactCreateDocumentClickObservation,
  beginExactCreateDocumentClickSettlement,
  consumeExactCreateDocumentClickObservationByPath,
  finalizeExactCreateDocumentClickObservation,
  readExactCreateDocumentClickObservation,
} from "./android-document-create-observation.mjs";

const CREATE_CLICK_SETTLEMENT_MS = 500;
const CREATE_TOUCH_TRANSPORT_LEASE_MS = 5000;

export {
  armExactCreateDocumentClickObservation,
  beginExactCreateDocumentClickSettlement,
  consumeExactCreateDocumentClickObservationByPath,
  finalizeExactCreateDocumentClickObservation,
  readExactCreateDocumentClickObservation,
} from "./android-document-create-observation.mjs";

export async function readExactCreateDocumentPointer(expectedPath) {
  const observe = () => {
    const candidates = [...document.querySelectorAll(
      '[data-deve-search-result-action="create-doc"]',
    )].filter((element) => {
      if (element.getAttribute("data-deve-search-result-create-target") !== expectedPath) {
        return false;
      }
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return style.display !== "none"
        && style.visibility !== "hidden"
        && rect.width > 0
        && rect.height > 0
        && rect.right > 0
        && rect.bottom > 0
        && rect.left < window.innerWidth
        && rect.top < window.innerHeight;
    });
    if (candidates.length !== 1) return { kind: "not-unique", count: candidates.length };
    const element = candidates[0];
    const rect = element.getBoundingClientRect();
    const point = {
      x: rect.left + Math.min(24, rect.width / 2),
      y: rect.top + Math.min(24, rect.height / 2),
    };
    const hit = document.elementFromPoint(point.x, point.y);
    if (!hit || (hit !== element && !element.contains(hit))) {
      return { kind: "occluded", count: 1 };
    }
    return {
      kind: "observed",
      count: 1,
      element,
      point,
      rect: { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
    };
  };
  const before = observe();
  if (before.kind !== "observed") return before;
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const after = observe();
  if (after.kind !== "observed") return after;
  const stable = before.element === after.element
    && Math.abs(before.rect.left - after.rect.left) < 0.5
    && Math.abs(before.rect.top - after.rect.top) < 0.5
    && Math.abs(before.rect.width - after.rect.width) < 0.5
    && Math.abs(before.rect.height - after.rect.height) < 0.5;
  return stable
    ? { kind: "ready", count: 1, point: after.point }
    : { kind: "moving", count: 1 };
}

async function waitForExactCreateDocumentClickSettlement(page, token) {
  const attempts = 21;
  const intervalMs = 25;
  let observation = null;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    observation = await page.call(readExactCreateDocumentClickObservation, token);
    if (observation?.kind !== "observed" || observation.clicked || observation.blocked) {
      return observation;
    }
    if (attempt + 1 < attempts) {
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  }
  return observation;
}

export async function clickExactCreateDocument(
  page,
  path,
  expectedWriterScope,
  tap = tapWebViewPoint,
) {
  const attemptNonce = `${Date.now()}-${clickExactCreateDocument.nextAttemptId}`;
  clickExactCreateDocument.nextAttemptId += 1;
  const target = await page.call(readExactCreateDocumentPointer, path);
  if (target?.kind !== "ready" || target.count !== 1 || !target.point) {
    throw new Error(`exact Create target is not stable and visible: ${JSON.stringify(target)}`);
  }
  let armed = null;
  let pointerError = null;
  let armRequestStarted = false;
  let armResponseObserved = false;
  try {
    await tap(page, target.point, {
      beforeContact: async () => {
        const refreshed = await page.call(readExactCreateDocumentPointer, path);
        if (refreshed?.kind !== "ready" || refreshed.count !== 1 || !refreshed.point) {
          throw new Error(
            `exact Create target was not stable before native touch contact: ${JSON.stringify(refreshed)}`,
          );
        }
        armRequestStarted = true;
        armed = await page.call(
          armExactCreateDocumentClickObservation,
          path,
          refreshed.point,
          attemptNonce,
          CREATE_TOUCH_TRANSPORT_LEASE_MS,
          expectedWriterScope,
        );
        armResponseObserved = true;
        if (armed?.kind !== "armed" || !Number.isSafeInteger(armed.token)) {
          throw new Error(
            `exact Create target changed before native touch contact: ${JSON.stringify(armed)}`,
          );
        }
        return refreshed.point;
      },
    });
  } catch (error) {
    pointerError = error;
  }
  if (armed?.kind !== "armed") {
    let cleanup = { kind: "not-owned", clicked: false };
    if (armRequestStarted && !armResponseObserved) {
      try {
        cleanup = await page.call(
          consumeExactCreateDocumentClickObservationByPath,
          path,
          attemptNonce,
        );
      } catch (cleanupError) {
        throw new Error(
          `${pointerError?.message ?? "Create pointer arm was not acknowledged"}; `
            + `unconfirmed_arm_cleanup=${cleanupError.message}`,
        );
      }
    }
    if (pointerError) {
      throw new Error(`${pointerError.message}; unconfirmed_arm_cleanup=${JSON.stringify(cleanup)}`);
    }
    throw new Error("Create pointer driver skipped before-contact identity admission");
  }
  let settlement = null;
  let settlementError = null;
  try {
    const started = await page.call(
      beginExactCreateDocumentClickSettlement,
      armed.token,
      CREATE_CLICK_SETTLEMENT_MS,
    );
    if (started?.kind !== "settling") {
      throw new Error(`Create click settlement did not start: ${JSON.stringify(started)}`);
    }
    settlement = await waitForExactCreateDocumentClickSettlement(page, armed.token);
  } catch (error) {
    settlementError = error;
  }
  let observation;
  try {
    observation = await page.call(finalizeExactCreateDocumentClickObservation, armed.token);
  } catch (cleanupError) {
    throw new Error(
      `${pointerError?.message ?? "Create pointer observation cleanup failed"}; `
        + `observation_cleanup=${cleanupError.message}`,
    );
  }
  if (settlementError) {
    throw new Error(
      `${pointerError?.message ?? "Create click settlement failed"}; `
        + `settlement=${settlementError.message}; observation_cleanup=${JSON.stringify(observation)}`,
    );
  }
  if (pointerError) {
    throw new Error(`${pointerError.message}; click_observation=${JSON.stringify(observation)}`);
  }
  if (observation?.kind !== "observed" || observation.clicked !== true) {
    const category = settlement?.kind === "observed" && settlement.blocked !== true
      ? "click settlement timed out"
      : "did not produce a DOM click";
    throw new Error(
      `exact Create pointer ${category}: ${JSON.stringify(settlement)}; `
        + `observation_cleanup=${JSON.stringify(observation)}`,
    );
  }
  return observation;
}

clickExactCreateDocument.nextAttemptId = 1;

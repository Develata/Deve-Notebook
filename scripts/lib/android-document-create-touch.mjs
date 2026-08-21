import { tapWebViewPoint } from "./android-webview-pointer.mjs";
import {
  armExactCreateDocumentClickObservation,
  beginExactCreateDocumentClickSettlement,
  consumeExactCreateDocumentClickObservationByPath,
  finalizeExactCreateDocumentClickObservation,
  readExactCreateDocumentClickObservation,
  readExactCreateDocumentLateClick,
  waitExactCreateDocumentClickSettlement,
} from "./android-document-create-observation.mjs";

const CREATE_CLICK_SETTLEMENT_MS = 2000;
const CREATE_TOUCH_TRANSPORT_LEASE_MS = 5000;
const CREATE_LATE_CLICK_DIAGNOSTIC_MS = 8000;
const CREATE_TARGET_OBSERVATION_FAILED = "android_document_create_target_observation_failed";
const CREATE_NATIVE_TOUCH_FAILED = "android_document_create_native_touch_failed";
const CREATE_SETTLEMENT_TRANSPORT_FAILED =
  "android_document_create_click_settlement_transport_failed";
const CREATE_OBSERVATION_CLEANUP_FAILED =
  "android_document_create_observation_cleanup_failed";

function projectReadyTarget(value) {
  if (value?.kind !== "ready" || value.count !== 1) return null;
  const x = value.point?.x;
  const y = value.point?.y;
  if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || y < 0) return null;
  return { point: { x, y } };
}

function projectCreateObservation(value) {
  if (value?.kind !== "observed"
    || typeof value.clicked !== "boolean"
    || typeof value.blocked !== "boolean"
    || (value.clicked && value.blocked)
    || value.laneSealed !== true) {
    return null;
  }
  const inputPhases = {};
  for (const phase of ["touchstart", "touchend", "pointerdown", "pointerup", "click"]) {
    if (typeof value.inputPhases?.[phase] !== "boolean") return null;
    inputPhases[phase] = value.inputPhases[phase];
  }
  if (inputPhases.click !== value.clicked) return null;
  let clickState = null;
  if (value.clicked) {
    const scopeNonce = /^(?:0|[1-9][0-9]*)$/.test(value.clickState?.scopeNonceRaw ?? "")
      ? Number(value.clickState.scopeNonceRaw)
      : null;
    if (value.clickState?.syncStatus !== "ready"
      || value.clickState.repoIdPresent !== true
      || !Number.isSafeInteger(scopeNonce)
      || scopeNonce <= 0) {
      return null;
    }
    clickState = { syncStatus: "ready", repoIdPresent: true, scopeNonce };
  } else if (value.clickState !== null) {
    return null;
  }
  const scrollEvents = Number.isSafeInteger(value?.scrollEvidence?.scrollEvents)
    && value.scrollEvidence.scrollEvents >= 0
    ? value.scrollEvidence.scrollEvents
    : null;
  const projectScrollTop = (offset) => offset === null
    || (Number.isFinite(offset) && Math.abs(offset) <= 10_000_000)
    ? offset
    : undefined;
  const documentScrollTopAtArm = projectScrollTop(
    value?.scrollEvidence?.documentScrollTopAtArm,
  );
  const targetScrollerTopAtArm = projectScrollTop(
    value?.scrollEvidence?.targetScrollerTopAtArm,
  );
  if (scrollEvents === null
    || documentScrollTopAtArm === undefined
    || targetScrollerTopAtArm === undefined) return null;
  return {
    kind: "observed",
    clicked: value.clicked,
    blocked: value.blocked,
    clickState,
    inputPhases,
    scrollEvidence: {
      scrollEvents,
      documentScrollTopAtArm,
      targetScrollerTopAtArm,
    },
    laneSealed: true,
  };
}

function createObservationsAgree(settlement, finalObservation) {
  const clickStateAgrees = settlement.clickState === null
    ? finalObservation.clickState === null
    : finalObservation.clickState !== null
      && settlement.clickState.syncStatus === finalObservation.clickState.syncStatus
      && settlement.clickState.repoIdPresent === finalObservation.clickState.repoIdPresent
      && settlement.clickState.scopeNonce === finalObservation.clickState.scopeNonce;
  return settlement.clicked === finalObservation.clicked
    && settlement.blocked === finalObservation.blocked
    && clickStateAgrees
    && Object.keys(settlement.inputPhases).every(
      (phase) => settlement.inputPhases[phase] === finalObservation.inputPhases[phase],
    )
    && settlement.scrollEvidence.documentScrollTopAtArm
      === finalObservation.scrollEvidence.documentScrollTopAtArm
    && settlement.scrollEvidence.targetScrollerTopAtArm
      === finalObservation.scrollEvidence.targetScrollerTopAtArm
    && finalObservation.scrollEvidence.scrollEvents
      >= settlement.scrollEvidence.scrollEvents;
}

function projectLateClickEvidence(value) {
  if (value?.kind !== "observed"
    || typeof value.laneSealed !== "boolean"
    || typeof value.lateClickObserved !== "boolean"
    || !(value.lateClickDelayMs === null
      || (Number.isSafeInteger(value.lateClickDelayMs)
        && value.lateClickDelayMs >= 0
        && value.lateClickDelayMs <= 60_000))) {
    return {
      kind: "invalid",
      laneSealed: false,
      lateClickObserved: false,
      lateClickDelayMs: null,
    };
  }
  const delay = Number.isSafeInteger(value?.lateClickDelayMs)
    && value.lateClickDelayMs >= 0
    && value.lateClickDelayMs <= 60_000
    ? value.lateClickDelayMs
    : null;
  return {
    kind: "observed",
    laneSealed: value.laneSealed,
    lateClickObserved: value.lateClickObserved,
    lateClickDelayMs: delay,
  };
}

export {
  armExactCreateDocumentClickObservation,
  beginExactCreateDocumentClickSettlement,
  consumeExactCreateDocumentClickObservationByPath,
  finalizeExactCreateDocumentClickObservation,
  readExactCreateDocumentClickObservation,
  readExactCreateDocumentLateClick,
  waitExactCreateDocumentClickSettlement,
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

async function waitForLateCreateDocumentClick(page, timeoutMs) {
  const intervalMs = 100;
  const attempts = Math.ceil(timeoutMs / intervalMs) + 1;
  let evidence = null;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      evidence = projectLateClickEvidence(
        await page.call(readExactCreateDocumentLateClick),
      );
    } catch {
      break;
    }
    if (evidence?.lateClickObserved) return evidence;
    if (attempt + 1 < attempts) {
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  }
  return evidence;
}

export async function clickExactCreateDocument(
  page,
  path,
  expectedWriterScope,
  tap = tapWebViewPoint,
  lateClickDiagnosticMs = CREATE_LATE_CLICK_DIAGNOSTIC_MS,
) {
  const attemptNonce = `${Date.now()}-${clickExactCreateDocument.nextAttemptId}`;
  clickExactCreateDocument.nextAttemptId += 1;
  let target;
  try {
    target = projectReadyTarget(await page.call(readExactCreateDocumentPointer, path));
  } catch {
    throw new Error(CREATE_TARGET_OBSERVATION_FAILED);
  }
  if (!target) throw new Error(CREATE_TARGET_OBSERVATION_FAILED);
  let armed = null;
  let pointerError = null;
  let armRequestStarted = false;
  let armResponseObserved = false;
  try {
    await tap(page, target.point, {
      beforeContact: async () => {
        const refreshed = projectReadyTarget(
          await page.call(readExactCreateDocumentPointer, path),
        );
        if (!refreshed) throw new Error(CREATE_TARGET_OBSERVATION_FAILED);
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
        if (armed?.kind !== "armed" || !Number.isSafeInteger(armed.token) || armed.token <= 0) {
          throw new Error(CREATE_NATIVE_TOUCH_FAILED);
        }
        return refreshed.point;
      },
    });
  } catch {
    pointerError = new Error(CREATE_NATIVE_TOUCH_FAILED);
  }
  if (armed?.kind !== "armed") {
    if (armRequestStarted && !armResponseObserved) {
      try {
        await page.call(
          consumeExactCreateDocumentClickObservationByPath,
          path,
          attemptNonce,
        );
      } catch {
        if (pointerError) {
          throw new Error(
            `${CREATE_NATIVE_TOUCH_FAILED}; secondary=${CREATE_OBSERVATION_CLEANUP_FAILED}`,
          );
        }
        throw new Error(CREATE_OBSERVATION_CLEANUP_FAILED);
      }
    }
    if (pointerError) {
      throw new Error(CREATE_NATIVE_TOUCH_FAILED);
    }
    throw new Error("Create pointer driver skipped before-contact identity admission");
  }
  let settlement = null;
  let settlementError = false;
  try {
    const started = await page.call(
      beginExactCreateDocumentClickSettlement,
      armed.token,
      CREATE_CLICK_SETTLEMENT_MS,
    );
    if (started?.kind !== "settling" || started.token !== armed.token) {
      throw new Error(CREATE_SETTLEMENT_TRANSPORT_FAILED);
    }
    settlement = projectCreateObservation(
      await page.call(waitExactCreateDocumentClickSettlement, armed.token),
    );
    if (!settlement) {
      throw new Error(CREATE_SETTLEMENT_TRANSPORT_FAILED);
    }
  } catch {
    settlementError = true;
  }
  let lateClickEvidence = null;
  if (!settlementError
    && !pointerError
    && settlement?.kind === "observed"
    && settlement.clicked !== true
    && settlement.blocked !== true
    && lateClickDiagnosticMs > 0) {
    lateClickEvidence = await waitForLateCreateDocumentClick(page, lateClickDiagnosticMs);
  }
  let observation;
  try {
    observation = projectCreateObservation(
      await page.call(finalizeExactCreateDocumentClickObservation, armed.token),
    );
    if (!observation) {
      throw new Error(CREATE_OBSERVATION_CLEANUP_FAILED);
    }
  } catch {
    if (pointerError) throw new Error(CREATE_NATIVE_TOUCH_FAILED);
    if (settlementError) throw new Error(CREATE_SETTLEMENT_TRANSPORT_FAILED);
    throw new Error(CREATE_OBSERVATION_CLEANUP_FAILED);
  }
  if (settlementError) {
    throw new Error(
      `${CREATE_SETTLEMENT_TRANSPORT_FAILED}; `
        + `observation_cleanup=${JSON.stringify(observation)}`,
    );
  }
  if (pointerError) {
    throw new Error(CREATE_NATIVE_TOUCH_FAILED);
  }
  if (!createObservationsAgree(settlement, observation)) {
    throw new Error(CREATE_OBSERVATION_CLEANUP_FAILED);
  }
  if (observation.clicked
    && observation.clickState.scopeNonce !== expectedWriterScope.scopeNonce) {
    throw new Error(CREATE_OBSERVATION_CLEANUP_FAILED);
  }
  if (observation?.kind !== "observed" || observation.clicked !== true) {
    const category = settlement?.kind === "observed" && settlement.blocked !== true
      ? "click settlement timed out"
      : "did not produce a DOM click";
    throw new Error(
      `exact Create pointer ${category}: ${JSON.stringify(settlement)}; `
        + `observation_cleanup=${JSON.stringify(observation)}; `
        + `late_click=${JSON.stringify(lateClickEvidence)}`,
    );
  }
  return observation;
}

clickExactCreateDocument.nextAttemptId = 1;

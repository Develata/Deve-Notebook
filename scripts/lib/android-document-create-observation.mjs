export function armExactCreateDocumentClickObservation(
  expectedPath,
  point,
  attemptNonce,
  settlementTimeoutMs = 500,
  expectedWriterScope = null,
) {
  const readWriterScope = () => {
    const status = document.querySelector("[data-deve-sync-status]");
    return {
      syncStatus: status?.getAttribute("data-deve-sync-status") ?? null,
      repoIdRaw: status?.getAttribute("data-deve-repo-id") ?? null,
      scopeNonceRaw: status?.getAttribute("data-deve-scope-nonce") ?? null,
    };
  };
  const sameWriterScope = (left, right) => left.syncStatus === right.syncStatus
    && left.repoIdRaw === right.repoIdRaw
    && left.scopeNonceRaw === right.scopeNonceRaw;
  if (!point || !Number.isFinite(point.x) || !Number.isFinite(point.y)) {
    return { kind: "changed" };
  }
  if (typeof attemptNonce !== "string" || attemptNonce.length === 0) {
    return { kind: "attempt-invalid" };
  }
  if (!Number.isFinite(settlementTimeoutMs) || settlementTimeoutMs <= 0) {
    return { kind: "timeout-invalid" };
  }
  if (!expectedWriterScope
    || typeof expectedWriterScope.repoId !== "string"
    || expectedWriterScope.repoId.length === 0
    || !Number.isSafeInteger(expectedWriterScope.scopeNonce)
    || expectedWriterScope.scopeNonce <= 0) {
    return { kind: "expected-writer-scope-invalid" };
  }
  if (globalThis.__deveAndroidCreatePointerObservation) return { kind: "active" };
  const existingLane = globalThis.__deveAndroidCreatePointerLane;
  const lane = existingLane ?? {
    documentRoot: document,
    nextToken: 1,
    sealed: false,
    sealListener: null,
    lateClick: null,
    finalTouchEndObservedAtMs: null,
  };
  if (lane.documentRoot !== document) return { kind: "document-mismatch" };
  if (lane.sealed) return { kind: "sealed" };
  globalThis.__deveAndroidCreatePointerLane = lane;
  const hit = document.elementFromPoint(point.x, point.y);
  const target = hit?.closest?.('[data-deve-search-result-action="create-doc"]') ?? null;
  const exactTargets = [...document.querySelectorAll(
    '[data-deve-search-result-action="create-doc"]',
  )].filter((element) =>
    element.getAttribute("data-deve-search-result-create-target") === expectedPath);
  if (exactTargets.length !== 1 || exactTargets[0] !== target) {
    return { kind: "changed" };
  }
  const scrolling = document.scrollingElement ?? null;
  let targetScroller = target.parentElement ?? null;
  while (targetScroller && !(targetScroller.scrollHeight > targetScroller.clientHeight)) {
    targetScroller = targetScroller.parentElement ?? null;
  }
  const token = lane.nextToken;
  lane.nextToken += 1;
  const observation = {
    token,
    attemptNonce,
    expectedPath,
    clicked: false,
    blocked: false,
    clickState: null,
    inputPhases: {
      touchstart: false,
      touchend: false,
      pointerdown: false,
      pointerup: false,
      click: false,
    },
    touchEndObservedAtMs: null,
    scrollEvidence: {
      scrollEvents: 0,
      documentScrollTopAtArm: typeof scrolling?.scrollTop === "number"
        ? scrolling.scrollTop
        : null,
      targetScrollerTopAtArm: typeof targetScroller?.scrollTop === "number"
        ? targetScroller.scrollTop
        : null,
    },
    documentRoot: document,
    listener: null,
    phaseListeners: [],
    writerScope: readWriterScope(),
    expiresAt: Date.now() + settlementTimeoutMs,
    expiryTimer: null,
    settlementResolve: null,
  };
  if (observation.writerScope.syncStatus !== "ready"
    || typeof observation.writerScope.repoIdRaw !== "string"
    || observation.writerScope.repoIdRaw.length === 0
    || !/^[1-9][0-9]*$/.test(observation.writerScope.scopeNonceRaw ?? "")) {
    return { kind: "writer-scope-invalid" };
  }
  if (observation.writerScope.repoIdRaw !== expectedWriterScope.repoId
    || observation.writerScope.scopeNonceRaw !== String(expectedWriterScope.scopeNonce)) {
    return { kind: "writer-scope-changed" };
  }
  const sealLane = () => {
    lane.sealed = true;
    if (!lane.sealListener) {
      lane.sealListener = (event) => {
        const target = event.target
          ?.closest?.('[data-deve-search-result-action="create-doc"]') ?? null;
        if (!target) return;
        if (!lane.lateClick) {
          lane.lateClick = {
            observedAtMs: Date.now(),
          };
        }
        event.preventDefault();
        event.stopImmediatePropagation();
      };
      lane.documentRoot.addEventListener("click", lane.sealListener, { capture: true });
    }
  };
  const matchesCurrentTargetAndScope = (event) => {
    const eventTarget = event.target
      ?.closest?.('[data-deve-search-result-action="create-doc"]') ?? null;
    const currentExactTargets = [...document.querySelectorAll(
      '[data-deve-search-result-action="create-doc"]',
    )].filter((element) =>
      element.getAttribute("data-deve-search-result-create-target") === expectedPath);
    return currentExactTargets.length === 1
      && currentExactTargets[0] === eventTarget
      && sameWriterScope(observation.writerScope, readWriterScope());
  };
  for (const phase of ["touchstart", "touchend", "pointerdown", "pointerup"]) {
    const phaseListener = (event) => {
      if (globalThis.__deveAndroidCreatePointerObservation !== observation) return;
      if (matchesCurrentTargetAndScope(event)) {
        observation.inputPhases[phase] = true;
        if (phase === "touchend") {
          observation.touchEndObservedAtMs = Date.now();
        }
      }
    };
    observation.phaseListeners.push({ phase, listener: phaseListener });
    observation.documentRoot.addEventListener(phase, phaseListener, { capture: true });
  }
  const scrollListener = () => {
    if (globalThis.__deveAndroidCreatePointerObservation !== observation) return;
    observation.scrollEvidence.scrollEvents += 1;
  };
  observation.phaseListeners.push({ phase: "scroll", listener: scrollListener });
  observation.documentRoot.addEventListener("scroll", scrollListener, { capture: true });
  observation.cleanup = () => {
    observation.documentRoot.removeEventListener("click", observation.listener, { capture: true });
    for (const phaseListener of observation.phaseListeners) {
      observation.documentRoot.removeEventListener(
        phaseListener.phase,
        phaseListener.listener,
        { capture: true },
      );
    }
  };
  const listener = (event) => {
    if (globalThis.__deveAndroidCreatePointerObservation !== observation) return;
    if (Date.now() >= observation.expiresAt) {
      sealLane();
      event.preventDefault();
      event.stopImmediatePropagation();
      return;
    }
    const clickTarget = event.target
      ?.closest?.('[data-deve-search-result-action="create-doc"]') ?? null;
    const currentExactTargets = [...document.querySelectorAll(
      '[data-deve-search-result-action="create-doc"]',
    )].filter((element) =>
      element.getAttribute("data-deve-search-result-create-target") === expectedPath);
    const writerScope = readWriterScope();
    if (currentExactTargets.length !== 1
      || currentExactTargets[0] !== clickTarget
      || !sameWriterScope(observation.writerScope, writerScope)) {
      observation.blocked = true;
      sealLane();
      observation.settlementResolve?.();
      event.preventDefault();
      event.stopImmediatePropagation();
      return;
    }
    observation.clicked = true;
    observation.inputPhases.click = true;
    observation.clickState = {
      syncStatus: writerScope.syncStatus,
      repoIdPresent: Boolean(writerScope.repoIdRaw),
      scopeNonceRaw: writerScope.scopeNonceRaw,
    };
    sealLane();
    observation.settlementResolve?.();
  };
  observation.listener = listener;
  globalThis.__deveAndroidCreatePointerObservation = observation;
  document.addEventListener("click", listener, { capture: true, once: true });
  observation.expiryTimer = setTimeout(() => {
    if (globalThis.__deveAndroidCreatePointerObservation !== observation) return;
    sealLane();
    observation.cleanup();
    lane.finalObservation = {
      kind: "observed",
      token: observation.token,
      clicked: observation.clicked,
      blocked: observation.blocked,
      clickState: observation.clickState,
      inputPhases: { ...observation.inputPhases },
      scrollEvidence: { ...observation.scrollEvidence },
      laneSealed: lane.sealed === true,
    };
    lane.finalTouchEndObservedAtMs = observation.touchEndObservedAtMs;
    delete globalThis.__deveAndroidCreatePointerObservation;
    observation.settlementResolve?.();
  }, settlementTimeoutMs);
  return { kind: "armed", token };
}

export function readExactCreateDocumentClickObservation(token) {
  const lane = globalThis.__deveAndroidCreatePointerLane;
  const observation = globalThis.__deveAndroidCreatePointerObservation;
  if (!observation || observation.token !== token) {
    const finalObservation = lane?.finalObservation;
    return finalObservation?.token === token
      ? { ...finalObservation }
      : { kind: "missing", clicked: false };
  }
  return {
    kind: "observed",
    clicked: observation.clicked,
    blocked: observation.blocked,
    clickState: observation.clickState,
    inputPhases: { ...observation.inputPhases },
    scrollEvidence: { ...observation.scrollEvidence },
    laneSealed: lane?.sealed === true,
  };
}

export function beginExactCreateDocumentClickSettlement(token, settlementTimeoutMs) {
  if (!Number.isFinite(settlementTimeoutMs) || settlementTimeoutMs <= 0) {
    return { kind: "timeout-invalid" };
  }
  const lane = globalThis.__deveAndroidCreatePointerLane;
  const observation = globalThis.__deveAndroidCreatePointerObservation;
  if (lane?.documentRoot !== document || !observation || observation.token !== token) {
    return { kind: "missing" };
  }
  const expire = () => {
    if (globalThis.__deveAndroidCreatePointerObservation !== observation) return;
    lane.sealed = true;
    if (!lane.sealListener) {
      lane.sealListener = (event) => {
        const target = event.target
          ?.closest?.('[data-deve-search-result-action="create-doc"]') ?? null;
        if (!target) return;
        if (!lane.lateClick) {
          lane.lateClick = {
            observedAtMs: Date.now(),
          };
        }
        event.preventDefault();
        event.stopImmediatePropagation();
      };
      lane.documentRoot.addEventListener("click", lane.sealListener, { capture: true });
    }
    observation.cleanup();
    lane.finalObservation = {
      kind: "observed",
      token: observation.token,
      clicked: observation.clicked,
      blocked: observation.blocked,
      clickState: observation.clickState,
      inputPhases: { ...observation.inputPhases },
      scrollEvidence: { ...observation.scrollEvidence },
      laneSealed: lane.sealed === true,
    };
    lane.finalTouchEndObservedAtMs = observation.touchEndObservedAtMs;
    delete globalThis.__deveAndroidCreatePointerObservation;
    observation.settlementResolve?.();
  };
  clearTimeout(observation.expiryTimer);
  observation.expiresAt = Date.now() + settlementTimeoutMs;
  observation.expiryTimer = setTimeout(expire, settlementTimeoutMs);
  return { kind: "settling", token };
}

export function waitExactCreateDocumentClickSettlement(token) {
  const lane = globalThis.__deveAndroidCreatePointerLane;
  const observation = globalThis.__deveAndroidCreatePointerObservation;
  if (lane?.documentRoot !== document || !observation || observation.token !== token) {
    const finalObservation = lane?.finalObservation;
    return finalObservation?.token === token
      ? { ...finalObservation }
      : { kind: "missing", clicked: false };
  }
  if (observation.settlementResolve) {
    return { kind: "wait-active", clicked: false };
  }
  const snapshot = () => ({
    kind: "observed",
    clicked: observation.clicked,
    blocked: observation.blocked,
    clickState: observation.clickState,
    inputPhases: { ...observation.inputPhases },
    scrollEvidence: { ...observation.scrollEvidence },
    laneSealed: lane.sealed === true,
  });
  if (observation.clicked || observation.blocked) return snapshot();
  return new Promise((resolve) => {
    const finish = () => {
      if (observation.settlementResolve !== finish) return;
      observation.settlementResolve = null;
      resolve(snapshot());
    };
    observation.settlementResolve = finish;
    if (observation.clicked || observation.blocked) finish();
  });
}

export function finalizeExactCreateDocumentClickObservation(token) {
  const lane = globalThis.__deveAndroidCreatePointerLane;
  if (lane?.documentRoot === document) {
    lane.sealed = true;
    if (!lane.sealListener) {
      lane.sealListener = (event) => {
        const target = event.target
          ?.closest?.('[data-deve-search-result-action="create-doc"]') ?? null;
        if (!target) return;
        if (!lane.lateClick) {
          lane.lateClick = {
            observedAtMs: Date.now(),
          };
        }
        event.preventDefault();
        event.stopImmediatePropagation();
      };
      lane.documentRoot.addEventListener("click", lane.sealListener, { capture: true });
    }
  }
  const observation = globalThis.__deveAndroidCreatePointerObservation;
  if (!observation || observation.token !== token) {
    const finalObservation = lane?.finalObservation;
    return finalObservation?.token === token
      ? { ...finalObservation, laneSealed: lane?.sealed === true }
      : { kind: "missing", clicked: false, laneSealed: lane?.sealed === true };
  }
  clearTimeout(observation.expiryTimer);
  observation.cleanup();
  delete globalThis.__deveAndroidCreatePointerObservation;
  observation.settlementResolve?.();
  return {
    kind: "observed",
    clicked: observation.clicked,
    blocked: observation.blocked,
    clickState: observation.clickState,
    inputPhases: { ...observation.inputPhases },
    scrollEvidence: { ...observation.scrollEvidence },
    laneSealed: lane?.sealed === true,
  };
}

export function readExactCreateDocumentLateClick() {
  const lane = globalThis.__deveAndroidCreatePointerLane;
  if (!lane || lane.documentRoot !== document) {
    return { kind: "missing", lateClickObserved: false, lateClickDelayMs: null };
  }
  const lateClick = lane.lateClick ?? null;
  const rawDelay = lateClick
    && typeof lateClick.observedAtMs === "number"
    && typeof lane.finalTouchEndObservedAtMs === "number"
    ? lateClick.observedAtMs - lane.finalTouchEndObservedAtMs
    : null;
  const lateClickDelayMs = Number.isFinite(rawDelay)
    && rawDelay >= 0
    && rawDelay <= 60_000
    ? Math.round(rawDelay)
    : null;
  return {
    kind: "observed",
    laneSealed: lane.sealed === true,
    lateClickObserved: Boolean(lateClick),
    lateClickDelayMs,
  };
}

export function consumeExactCreateDocumentClickObservationByPath(expectedPath, attemptNonce) {
  const observation = globalThis.__deveAndroidCreatePointerObservation;
  if (!observation) return { kind: "missing", clicked: false };
  if (observation.expectedPath !== expectedPath || observation.attemptNonce !== attemptNonce) {
    return { kind: "owner-mismatch", clicked: false };
  }
  clearTimeout(observation.expiryTimer);
  observation.cleanup();
  delete globalThis.__deveAndroidCreatePointerObservation;
  observation.settlementResolve?.();
  return {
    kind: "observed",
    clicked: observation.clicked,
    blocked: observation.blocked,
    clickState: observation.clickState,
    inputPhases: { ...observation.inputPhases },
  };
}

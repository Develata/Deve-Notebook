import assert from "node:assert/strict";
import test from "node:test";

import {
  clickExactCreateDocument,
  readExactCreateDocumentClickObservation,
  readExactCreateDocumentLateClick,
  waitExactCreateDocumentClickSettlement,
} from "./lib/android-document-create-touch.mjs";
import {
  createResult,
  withCreateDom,
} from "./lib/android-document-create-pointer-fixture.mjs";

const exactPath = "notes/exact.md";
const admittedWriterScope = { repoId: "repo-1", scopeNonce: 7 };
const secretSentinel = "secret=/private/runner/device-path";

function closedObservation(clicked = false) {
  return {
    kind: "observed",
    clicked,
    blocked: false,
    clickState: clicked
      ? { syncStatus: "ready", repoIdPresent: true, scopeNonceRaw: "7" }
      : null,
    inputPhases: {
      touchstart: clicked,
      touchend: clicked,
      pointerdown: clicked,
      pointerup: clicked,
      click: clicked,
    },
    scrollEvidence: {
      scrollEvents: 0,
      documentScrollTopAtArm: 0,
      targetScrollerTopAtArm: null,
    },
    laneSealed: true,
  };
}

test("exact Create settlement waits inside the WebView without host polling", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    let pageSideWaits = 0;
    let observationReads = 0;
    const page = {
      async call(fn, ...args) {
        if (fn === waitExactCreateDocumentClickSettlement) pageSideWaits += 1;
        if (fn === readExactCreateDocumentClickObservation) observationReads += 1;
        return fn(...args);
      },
      async send(_method, params) {
        if (params.type === "touchEnd") setTimeout(() => exact.emitClick(), 25);
      },
    };

    const observation = await clickExactCreateDocument(page, exactPath, admittedWriterScope);

    assert.equal(observation.clicked, true);
    assert.equal(pageSideWaits, 1);
    assert.equal(observationReads, 0);
  });
});

test("Create settlement transport failure exposes only a fixed category", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    const page = {
      async call(fn, ...args) {
        if (fn === waitExactCreateDocumentClickSettlement) throw new Error(secretSentinel);
        return fn(...args);
      },
      async send() {},
    };

    const failure = await clickExactCreateDocument(page, exactPath, admittedWriterScope, undefined, 0)
      .then(() => null, (error) => error);

    assert.match(failure.message, /android_document_create_click_settlement_transport_failed/);
    assert.doesNotMatch(failure.message, /secret=|private|device-path/);
  });
});

test("Create observation cleanup failure exposes only a fixed category", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    const page = {
      async call(fn, ...args) {
        if (fn.name === "finalizeExactCreateDocumentClickObservation") {
          throw new Error(secretSentinel);
        }
        return fn(...args);
      },
      async send(_method, params) {
        if (params.type === "touchEnd") exact.emitClick();
      },
    };

    const failure = await clickExactCreateDocument(page, exactPath, admittedWriterScope)
      .then(() => null, (error) => error);

    assert.equal(failure.message, "android_document_create_observation_cleanup_failed");
  });
});

test("Create native touch failure exposes only a fixed category", async () => {
  const page = {
    async call(fn) {
      if (fn.name === "readExactCreateDocumentPointer") {
        return { kind: "ready", count: 1, point: { x: 17, y: 23 } };
      }
      if (fn.name === "armExactCreateDocumentClickObservation") return { kind: "armed", token: 7 };
      if (fn.name === "beginExactCreateDocumentClickSettlement") {
        return { kind: "settling", token: 7 };
      }
      if (fn === waitExactCreateDocumentClickSettlement) {
        return closedObservation();
      }
      return closedObservation();
    },
  };

  const failure = await clickExactCreateDocument(
    page,
    exactPath,
    admittedWriterScope,
    async (_page, _point, { beforeContact }) => {
      await beforeContact();
      throw new Error(secretSentinel);
    },
    0,
  ).then(() => null, (error) => error);

  assert.equal(failure.message, "android_document_create_native_touch_failed");
});

test("Create target observation failure exposes only a fixed category", async () => {
  const page = {
    async call() { throw new Error(secretSentinel); },
  };

  const failure = await clickExactCreateDocument(page, exactPath, admittedWriterScope)
    .then(() => null, (error) => error);

  assert.equal(failure.message, "android_document_create_target_observation_failed");
});

test("Create malformed renderer returns expose only closed evidence", async () => {
  const tapAfterAdmission = async (_page, _point, { beforeContact }) => {
    await beforeContact();
  };
  const runCase = async ({ malformed, lateClickDiagnosticMs = 0 }) => {
    const page = {
      async call(fn) {
        if (fn.name === "readExactCreateDocumentPointer") {
          return malformed === "target"
            ? { kind: "malformed", detail: secretSentinel }
            : { kind: "ready", count: 1, point: { x: 17, y: 23 } };
        }
        if (fn.name === "armExactCreateDocumentClickObservation") {
          return { kind: "armed", token: 7 };
        }
        if (fn.name === "beginExactCreateDocumentClickSettlement") {
          return { kind: "settling", token: 7 };
        }
        if (fn.name === "waitExactCreateDocumentClickSettlement") {
          if (malformed === "settlement") {
            return { kind: "malformed", detail: secretSentinel };
          }
          return {
            ...closedObservation(malformed === "finalize"),
          };
        }
        if (fn.name === "readExactCreateDocumentLateClick") {
          return {
            kind: "observed",
            laneSealed: true,
            lateClickObserved: true,
            lateClickDelayMs: 17,
            detail: secretSentinel,
          };
        }
        if (fn.name === "finalizeExactCreateDocumentClickObservation") {
          return malformed === "finalize"
            ? { kind: "malformed", detail: secretSentinel }
            : closedObservation();
        }
        return { kind: "malformed", detail: secretSentinel };
      },
    };
    return clickExactCreateDocument(
      page,
      exactPath,
      admittedWriterScope,
      tapAfterAdmission,
      lateClickDiagnosticMs,
    ).then(() => null, (error) => error);
  };

  for (const malformed of ["target", "settlement", "finalize", "late-click"]) {
    const failure = await runCase({
      malformed,
      lateClickDiagnosticMs: malformed === "late-click" ? 1 : 0,
    });
    assert.ok(failure, `${malformed} must fail closed`);
    assert.doesNotMatch(failure.message, /secret=|private|runner|device-path|detail/);
  }
});

test("Create contradictory observed evidence always fails closed", async () => {
  const completeClick = closedObservation(true);
  const incomplete = { kind: "observed", clicked: false, blocked: false, laneSealed: true };
  const wrongType = { ...closedObservation(), clicked: "false" };
  const blockedClick = { ...completeClick, blocked: true };
  const phaseMismatch = {
    ...completeClick,
    inputPhases: { ...completeClick.inputPhases, click: false },
  };
  const unsealed = { ...completeClick, laneSealed: false };
  const wrongScope = {
    ...completeClick,
    clickState: { ...completeClick.clickState, scopeNonceRaw: "8" },
  };
  const cases = [
    { settlement: incomplete, finalObservation: closedObservation() },
    { settlement: wrongType, finalObservation: closedObservation() },
    { settlement: blockedClick, finalObservation: completeClick },
    { settlement: phaseMismatch, finalObservation: completeClick },
    { settlement: closedObservation(), finalObservation: completeClick },
    { settlement: completeClick, finalObservation: unsealed },
    { settlement: wrongScope, finalObservation: wrongScope },
  ];

  for (const [index, evidence] of cases.entries()) {
    const page = {
      async call(fn) {
        if (fn.name === "readExactCreateDocumentPointer") {
          return { kind: "ready", count: 1, point: { x: 17, y: 23 } };
        }
        if (fn.name === "armExactCreateDocumentClickObservation") {
          return { kind: "armed", token: 7 };
        }
        if (fn.name === "beginExactCreateDocumentClickSettlement") {
          return { kind: "settling", token: 7 };
        }
        if (fn.name === "waitExactCreateDocumentClickSettlement") {
          return evidence.settlement;
        }
        if (fn.name === "finalizeExactCreateDocumentClickObservation") {
          return evidence.finalObservation;
        }
        return { kind: "malformed", detail: secretSentinel };
      },
    };
    const failure = await clickExactCreateDocument(
      page,
      exactPath,
      admittedWriterScope,
      async (_page, _point, { beforeContact }) => beforeContact(),
      0,
    ).then(() => null, (error) => error);
    assert.ok(failure, `contradictory observed evidence ${index} must fail closed`);
  }
});

test("Create native touch remains the primary category when compensation cleanup also fails", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    const page = {
      async call(fn, ...args) {
        if (fn.name === "armExactCreateDocumentClickObservation") {
          fn(...args);
          throw new Error(secretSentinel);
        }
        if (fn.name === "consumeExactCreateDocumentClickObservationByPath") {
          throw new Error(secretSentinel);
        }
        return fn(...args);
      },
    };
    const failure = await clickExactCreateDocument(
      page,
      exactPath,
      admittedWriterScope,
      async (_page, _point, { beforeContact }) => beforeContact(),
      0,
    ).then(() => null, (error) => error);

    assert.equal(
      failure.message,
      "android_document_create_native_touch_failed; "
        + "secondary=android_document_create_observation_cleanup_failed",
    );
    assert.doesNotMatch(failure.message, /secret=|private|runner|device-path/);
  });
});

test("exact Create waits up to the bounded settlement deadline for a delayed click", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    const sent = [];
    const page = {
      async call(fn, ...args) { return fn(...args); },
      async send(_method, params) {
        sent.push(params.type);
        if (params.type === "touchEnd") setTimeout(() => exact.emitClick(), 750);
      },
    };

    const observation = await clickExactCreateDocument(page, exactPath, admittedWriterScope);

    assert.equal(observation.clicked, true);
    assert.deepEqual(sent, ["touchStart", "touchEnd"]);
  });
});

test("exact Create click settlement timeout never retransmits the committed touch", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    const sent = [];
    const page = {
      async call(fn, ...args) { return fn(...args); },
      async send(_method, params) { sent.push(params.type); },
    };

    await assert.rejects(
      clickExactCreateDocument(page, exactPath, admittedWriterScope, undefined, 25, 25),
      /click settlement timed out/,
    );

    assert.deepEqual(sent, ["touchStart", "touchEnd"]);
    assert.equal(globalThis.__deveAndroidCreatePointerObservation, undefined);
    assert.equal(globalThis.__deveAndroidCreatePointerLane.sealed, true);
    for (const phase of ["touchstart", "touchend", "pointerdown", "pointerup"]) {
      assert.equal(globalThis.document.listenerCount(phase), 0);
    }
    assert.equal(globalThis.document.listenerCount("scroll"), 0);
  });
});

test("committed-unknown Create seals the document lane against a late click and retry", async () => {
  let commits = 0;
  const exact = createResult(exactPath, { onCommit: () => { commits += 1; } });
  await withCreateDom([exact], exact, async () => {
    const sent = [];
    const page = {
      async call(fn, ...args) { return fn(...args); },
      async send(_method, params) { sent.push(params.type); },
    };

    await assert.rejects(
      clickExactCreateDocument(page, exactPath, admittedWriterScope, undefined, 25, 25),
      /settlement timed out/,
    );
    await assert.rejects(
      clickExactCreateDocument(page, exactPath, admittedWriterScope),
      /android_document_create_native_touch_failed/,
    );
    const late = exact.emitClick();

    assert.deepEqual(sent, ["touchStart", "touchEnd"]);
    assert.equal(late.immediatePropagationStopped, true);
    assert.equal(commits, 0);
    assert.equal(globalThis.document.clickListenerCount(), 1);
    const lateEvidence = readExactCreateDocumentLateClick();
    assert.equal(lateEvidence.kind, "observed");
    assert.equal(lateEvidence.laneSealed, true);
    assert.equal(lateEvidence.lateClickObserved, true);
    assert.equal(lateEvidence.lateClickDelayMs, null);
  });
});

test("exact Create accepts a delayed click beyond two seconds inside the bounded lease", async () => {
  let commits = 0;
  const exact = createResult(exactPath, { onCommit: () => { commits += 1; } });
  await withCreateDom([exact], exact, async () => {
    const page = {
      async call(fn, ...args) { return fn(...args); },
      async send(_method, params) {
        if (params.type === "touchStart") {
          exact.emitEvent("pointerdown");
          exact.emitEvent("touchstart");
        }
        if (params.type === "touchEnd") {
          exact.emitEvent("pointerup");
          exact.emitEvent("touchend");
          setTimeout(() => exact.emitClick(), 2300);
        }
      },
    };

    const observation = await clickExactCreateDocument(page, exactPath, admittedWriterScope);

    assert.equal(observation.clicked, true);
    assert.equal(commits, 1);
    assert.equal(globalThis.document.clickListenerCount(), 1);
  });
});

test("sealed late-click delay uses delivery wall clock when DOM timestamps are reused", async () => {
  let commits = 0;
  const exact = createResult(exactPath, { onCommit: () => { commits += 1; } });
  await withCreateDom([exact], exact, async () => {
    let resolveTouchEnd;
    const touchEnded = new Promise((resolve) => { resolveTouchEnd = resolve; });
    const page = {
      async call(fn, ...args) { return fn(...args); },
      async send(_method, params) {
        if (params.type === "touchEnd") {
          exact.emitEvent("touchend", 100);
          resolveTouchEnd();
        }
      },
    };

    const pending = clickExactCreateDocument(
      page,
      exactPath,
      admittedWriterScope,
      undefined,
      100,
      25,
    ).then(() => null, (error) => error);
    await touchEnded;
    await new Promise((resolve) => setTimeout(resolve, 50));
    exact.emitClick(100);
    const failure = await pending;

    assert.match(failure.message, /click settlement timed out/);
    assert.match(failure.message, /"lateClickObserved":true/);
    const delay = JSON.parse(failure.message.match(/late_click=(\{.*\})$/)[1]).lateClickDelayMs;
    assert.ok(delay >= 20, `expected delivery delay, got ${delay}`);
    assert.equal(commits, 0);
  });
});

import assert from "node:assert/strict";
import test from "node:test";

import {
  clickExactCreateDocument,
  readExactCreateDocumentLateClick,
} from "./lib/android-document-create-touch.mjs";
import {
  createResult,
  withCreateDom,
} from "./lib/android-document-create-pointer-fixture.mjs";

const exactPath = "notes/exact.md";
const admittedWriterScope = { repoId: "repo-1", scopeNonce: 7 };

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
      clickExactCreateDocument(page, exactPath, admittedWriterScope, undefined, 25),
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
      clickExactCreateDocument(page, exactPath, admittedWriterScope, undefined, 25),
      /settlement timed out/,
    );
    await assert.rejects(
      clickExactCreateDocument(page, exactPath, admittedWriterScope),
      /changed before native touch contact.*sealed/,
    );
    const late = exact.emitClick();

    assert.deepEqual(sent, ["touchStart", "touchEnd"]);
    assert.equal(late.immediatePropagationStopped, true);
    assert.equal(commits, 0);
    assert.equal(globalThis.document.clickListenerCount(), 1);
    const lateEvidence = readExactCreateDocumentLateClick();
    assert.equal(lateEvidence.kind, "observed");
    assert.equal(lateEvidence.laneSealed, true);
    assert.ok(lateEvidence.lateClick);
    assert.equal(lateEvidence.lateClickDelayMs, null);
  });
});

test("exact Create settlement timeout captures sealed late-click delay evidence", async () => {
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

    const failure = await clickExactCreateDocument(page, exactPath, admittedWriterScope)
      .then(() => null, (error) => error);

    assert.match(failure.message, /click settlement timed out/);
    assert.match(failure.message, /"lateClick":\{"at":\d+,"timeStamp":\d+\}/);
    assert.match(failure.message, /"lateClickDelayMs":\d+/);
    assert.equal(commits, 0);
    assert.equal(globalThis.document.clickListenerCount(), 1);
  });
});

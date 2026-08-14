import assert from "node:assert/strict";
import test from "node:test";
import vm from "node:vm";

import {
  armExactCreateDocumentClickObservation,
  beginExactCreateDocumentClickSettlement,
  clickExactCreateDocument,
  consumeExactCreateDocumentClickObservationByPath,
  finalizeExactCreateDocumentClickObservation,
  readExactCreateDocumentClickObservation,
  readExactCreateDocumentPointer,
} from "./lib/android-document-create-touch.mjs";
import { tapWebViewPoint } from "./lib/android-webview-pointer.mjs";
import {
  createResult,
  withCreateDom,
} from "./lib/android-document-create-pointer-fixture.mjs";

const exactPath = "notes/exact.md";
const admittedWriterScope = { repoId: "repo-1", scopeNonce: 7 };

test("observation functions execute in an isolated CDP serialization realm", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    const context = vm.createContext({
      document: globalThis.document,
      globalThis: {},
      Number,
      Date,
      setTimeout,
      clearTimeout,
    });
    const callSerialized = (fn, ...args) => vm.runInContext(
      `(${fn.toString()})(...${JSON.stringify(args)})`,
      context,
    );

    const armed = callSerialized(
      armExactCreateDocumentClickObservation,
      exactPath,
      { x: 34, y: 42 },
      "serialized-attempt",
      500,
      admittedWriterScope,
    );
    assert.equal(armed.kind, "armed");
    assert.equal(
      callSerialized(
        beginExactCreateDocumentClickSettlement,
        armed.token,
        500,
      ).kind,
      "settling",
    );
    assert.equal(
      callSerialized(readExactCreateDocumentClickObservation, armed.token).clicked,
      false,
    );
    const finalized = callSerialized(finalizeExactCreateDocumentClickObservation, armed.token);
    assert.equal(finalized.kind, "observed");
    assert.equal(finalized.laneSealed, true);

    const cleanupContext = vm.createContext({
      document: globalThis.document,
      globalThis: {},
      Number,
      Date,
      setTimeout,
      clearTimeout,
    });
    const cleanupCall = (fn, ...args) => vm.runInContext(
      `(${fn.toString()})(...${JSON.stringify(args)})`,
      cleanupContext,
    );
    assert.equal(
      cleanupCall(
        armExactCreateDocumentClickObservation,
        exactPath,
        { x: 34, y: 42 },
        "cleanup-attempt",
        500,
        admittedWriterScope,
      ).kind,
      "armed",
    );
    assert.equal(
      cleanupCall(
        consumeExactCreateDocumentClickObservationByPath,
        exactPath,
        "cleanup-attempt",
      ).kind,
      "observed",
    );
  });
});

test("a concurrent non-owner cannot clean the active Create observation", async () => {
  let commits = 0;
  const exact = createResult(exactPath, { onCommit: () => { commits += 1; } });
  await withCreateDom([exact], exact, async () => {
    let releaseFirstContact;
    const firstContact = new Promise((resolve) => { releaseFirstContact = resolve; });
    let firstArmed;
    const sent = [];
    const page = {
      async call(fn, ...args) { return fn(...args); },
      async send(_method, params) {
        sent.push(params.type);
        if (params.type === "touchStart") {
          await firstContact;
        } else if (params.type === "touchEnd") {
          exact.emitClick();
        }
      },
    };
    const first = clickExactCreateDocument(
      page,
      exactPath,
      admittedWriterScope,
      async (tapPage, point, { beforeContact }) => {
        const contact = await beforeContact();
        firstArmed = globalThis.__deveAndroidCreatePointerObservation;
        await tapWebViewPoint(tapPage, contact ?? point);
      },
    );
    while (!firstArmed) await new Promise((resolve) => setImmediate(resolve));

    await assert.rejects(
      clickExactCreateDocument(
        page,
        exactPath,
        admittedWriterScope,
        async (_page, _point, { beforeContact }) => beforeContact(),
      ),
      /changed before native touch contact.*active.*not-owned/,
    );
    assert.equal(globalThis.__deveAndroidCreatePointerObservation, firstArmed);
    releaseFirstContact();
    await first;

    assert.equal(commits, 1);
    assert.deepEqual(sent, ["touchStart", "touchEnd"]);
  });
});

test("touch transport lease is renewed only after touchEnd settlement begins", async () => {
  let commits = 0;
  const exact = createResult(exactPath, { onCommit: () => { commits += 1; } });
  await withCreateDom([exact], exact, async () => {
    const armed = armExactCreateDocumentClickObservation(
      exactPath,
      { x: 34, y: 42 },
      "transport-lease",
      100,
      admittedWriterScope,
    );
    assert.equal(armed.kind, "armed");
    await new Promise((resolve) => setTimeout(resolve, 60));
    assert.equal(
      beginExactCreateDocumentClickSettlement(armed.token, 100).kind,
      "settling",
    );
    await new Promise((resolve) => setTimeout(resolve, 60));
    exact.emitClick();
    const finalized = finalizeExactCreateDocumentClickObservation(armed.token);
    assert.equal(finalized.clicked, true);
    assert.equal(commits, 1);
  });
});

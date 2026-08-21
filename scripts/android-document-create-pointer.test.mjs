import assert from "node:assert/strict";
import test from "node:test";

import {
  armExactCreateDocumentClickObservation,
  beginExactCreateDocumentClickSettlement,
  clickExactCreateDocument,
  consumeExactCreateDocumentClickObservationByPath,
  finalizeExactCreateDocumentClickObservation,
  readExactCreateDocumentClickObservation,
  readExactCreateDocumentPointer,
  waitExactCreateDocumentClickSettlement,
} from "./lib/android-document-create-touch.mjs";
import { tapWebViewPoint } from "./lib/android-webview-pointer.mjs";
import {
  createResult,
  withCreateDom,
} from "./lib/android-document-create-pointer-fixture.mjs";

const exactPath = "notes/exact.md";
const admittedWriterScope = { repoId: "repo-1", scopeNonce: 7 };

function closedObservation(clicked = true) {
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

test("exact Create pointer ignores a stale result and requires a stable hit-tested target", async () => {
  const stale = createResult("Untitled.md");
  const exact = createResult(exactPath);
  await withCreateDom([stale, exact], exact, async () => {
    assert.deepEqual(
      await readExactCreateDocumentPointer(exactPath),
      { kind: "ready", count: 1, point: { x: 34, y: 42 } },
    );
  });
  await withCreateDom([exact, createResult(exactPath)], exact, async () => {
    assert.deepEqual(
      await readExactCreateDocumentPointer(exactPath),
      { kind: "not-unique", count: 2 },
    );
  });
  await withCreateDom([exact], stale, async () => {
    assert.deepEqual(
      await readExactCreateDocumentPointer(exactPath),
      { kind: "occluded", count: 1 },
    );
  });
});

test("exact Create pointer sends one native gesture only after identity admission", async () => {
  const page = {
    async call(fn, path) {
      if (fn === readExactCreateDocumentPointer) {
        assert.equal(path, exactPath);
        return { kind: "ready", count: 1, point: { x: 17, y: 23 } };
      }
      if (fn === armExactCreateDocumentClickObservation) {
        assert.equal(path, exactPath);
        return { kind: "armed", token: 7 };
      }
      if (fn === waitExactCreateDocumentClickSettlement) {
        assert.equal(path, 7);
        return closedObservation();
      }
      if (fn === beginExactCreateDocumentClickSettlement) {
        return { kind: "settling", token: 7 };
      }
      assert.equal(fn, finalizeExactCreateDocumentClickObservation);
      assert.equal(path, 7);
      return closedObservation();
    },
  };
  const taps = [];
  const observation = await clickExactCreateDocument(
    page,
    exactPath,
    admittedWriterScope,
    async (tapPage, point, { beforeContact }) => {
    taps.push({ tapPage, point, contactPoint: await beforeContact() });
    },
  );
  assert.deepEqual(taps, [{
    tapPage: page,
    point: { x: 17, y: 23 },
    contactPoint: { x: 17, y: 23 },
  }]);
  assert.deepEqual(observation.clickState, {
    syncStatus: "ready",
    repoIdPresent: true,
    scopeNonce: 7,
  });
  assert.deepEqual(observation.scrollEvidence, {
    scrollEvents: 0,
    documentScrollTopAtArm: 0,
    targetScrollerTopAtArm: null,
  });

  const changedPage = {
    async call(fn) {
      if (fn === readExactCreateDocumentPointer) {
        return { kind: "ready", count: 1, point: { x: 17, y: 23 } };
      }
      if (fn === consumeExactCreateDocumentClickObservationByPath) {
        return { kind: "missing", clicked: false };
      }
      return { kind: "changed" };
    },
  };
  await assert.rejects(
    clickExactCreateDocument(
      changedPage,
      exactPath,
      admittedWriterScope,
      async (_tapPage, _point, { beforeContact }) => beforeContact(),
    ),
    /android_document_create_native_touch_failed/,
  );
});

test("exact Create production wiring reports fixed native input phases and one gesture", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    const sent = [];
    const page = {
      async call(fn, ...args) { return fn(...args); },
      async send(method, params) {
        sent.push({ method, params });
        if (params.type === "touchStart") {
          exact.emitEvent("pointerdown");
          exact.emitEvent("touchstart");
        }
        if (params.type === "touchEnd") {
          exact.emitEvent("pointerup");
          exact.emitEvent("touchend");
          exact.emitClick();
        }
      },
    };

    const observation = await clickExactCreateDocument(page, exactPath, admittedWriterScope);

    assert.equal(observation.clicked, true);
    assert.deepEqual(observation.inputPhases, {
      touchstart: true,
      touchend: true,
      pointerdown: true,
      pointerup: true,
      click: true,
    });
    assert.deepEqual(sent.map(({ params }) => params.type), [
      "touchStart",
      "touchEnd",
    ]);
    for (const phase of ["touchstart", "touchend", "pointerdown", "pointerup"]) {
      assert.equal(globalThis.document.listenerCount(phase), 0);
    }
  });
});

test("final atomic observation confirms the page-side click settlement", async () => {
  const calls = [];
  const page = {
    async call(fn) {
      calls.push(fn);
      if (fn === readExactCreateDocumentPointer) {
        return { kind: "ready", count: 1, point: { x: 17, y: 23 } };
      }
      if (fn === armExactCreateDocumentClickObservation) return { kind: "armed", token: 7 };
      if (fn === waitExactCreateDocumentClickSettlement) {
        return closedObservation();
      }
      if (fn === beginExactCreateDocumentClickSettlement) {
        return { kind: "settling", token: 7 };
      }
      if (fn === finalizeExactCreateDocumentClickObservation) {
        return closedObservation();
      }
      throw new Error(`unexpected page function: ${fn.name}`);
    },
  };

  const observation = await clickExactCreateDocument(
    page,
    exactPath,
    admittedWriterScope,
    async (_page, _point, { beforeContact }) => beforeContact(),
  );

  assert.equal(observation.clicked, true);
  assert.equal(calls.filter((fn) => fn === finalizeExactCreateDocumentClickObservation).length, 1);
});

test("page-side expiry seals a Create when finalize never reaches the WebView", async () => {
  let commits = 0;
  const exact = createResult(exactPath, { onCommit: () => { commits += 1; } });
  await withCreateDom([exact], exact, async () => {
    const armed = armExactCreateDocumentClickObservation(
      exactPath,
      { x: 34, y: 42 },
      "lost-finalize",
      10,
      admittedWriterScope,
    );
    assert.equal(armed.kind, "armed");
    await new Promise((resolve) => setTimeout(resolve, 25));

    const late = exact.emitClick();
    const state = readExactCreateDocumentClickObservation(armed.token);
    const second = armExactCreateDocumentClickObservation(
      exactPath,
      { x: 34, y: 42 },
      "second-attempt",
      10,
      admittedWriterScope,
    );

    assert.equal(late.immediatePropagationStopped, true);
    assert.equal(commits, 0);
    assert.equal(state.kind, "observed");
    assert.equal(state.clicked, false);
    assert.equal(second.kind, "sealed");
  });
});

test("lost finalize transport still expires and seals inside the WebView", async () => {
  let commits = 0;
  const exact = createResult(exactPath, { onCommit: () => { commits += 1; } });
  await withCreateDom([exact], exact, async () => {
    const sent = [];
    const page = {
      async call(fn, ...args) {
        if (fn === waitExactCreateDocumentClickSettlement) {
          throw new Error("settlement transport lost");
        }
        if (fn === finalizeExactCreateDocumentClickObservation) {
          throw new Error("finalize never reached page");
        }
        return fn(...args);
      },
      async send(_method, params) { sent.push(params.type); },
    };

    await assert.rejects(
      clickExactCreateDocument(page, exactPath, admittedWriterScope),
      /android_document_create_click_settlement_transport_failed/,
    );
    await new Promise((resolve) => setTimeout(resolve, 2025));
    const late = exact.emitClick();
    const second = armExactCreateDocumentClickObservation(
      exactPath,
      { x: 34, y: 42 },
      "after-lost-finalize",
      10,
      admittedWriterScope,
    );

    assert.deepEqual(sent, ["touchStart", "touchEnd"]);
    assert.equal(late.immediatePropagationStopped, true);
    assert.equal(commits, 0);
    assert.equal(second.kind, "sealed");
    for (const phase of ["touchstart", "touchend", "pointerdown", "pointerup"]) {
      assert.equal(globalThis.document.listenerCount(phase), 0);
    }
  });
});

test("Create observation rejects an invalid writer scope before contact", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    globalThis.document.querySelector = () => ({
      getAttribute(name) {
        if (name === "data-deve-sync-status") return "ready";
        if (name === "data-deve-repo-id") return "repo-1";
        if (name === "data-deve-scope-nonce") return "0";
        return null;
      },
    });
    const result = armExactCreateDocumentClickObservation(
      exactPath,
      { x: 34, y: 42 },
      "invalid-writer",
      500,
      admittedWriterScope,
    );
    assert.equal(result.kind, "writer-scope-invalid");
    assert.equal(globalThis.__deveAndroidCreatePointerObservation, undefined);
  });
});

test("Create observation rejects a scope switch after quiet admission but before arm", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    globalThis.document.querySelector = () => ({
      getAttribute(name) {
        if (name === "data-deve-sync-status") return "ready";
        if (name === "data-deve-repo-id") return "repo-2";
        if (name === "data-deve-scope-nonce") return "8";
        return null;
      },
    });
    const result = armExactCreateDocumentClickObservation(
      exactPath,
      { x: 34, y: 42 },
      "scope-switched",
      500,
      admittedWriterScope,
    );
    assert.equal(result.kind, "writer-scope-changed");
    assert.equal(globalThis.__deveAndroidCreatePointerObservation, undefined);
  });
});

test("exact Create accepts a same-path remount before touch and contacts its new point", async () => {
  let committed = 0;
  const elements = [createResult(exactPath)];
  await withCreateDom(
    elements,
    (x) => x === elements[0].getLeft() + 24 ? elements[0] : null,
    async () => {
      const sent = [];
      const page = {
        async call(fn, ...args) { return fn(...args); },
        async send(_method, params) {
          sent.push({ type: params.type, x: params.touchPoints[0]?.x ?? null });
          if (params.type === "touchEnd") elements[0].emitClick();
        },
      };

      await clickExactCreateDocument(
        page,
        exactPath,
        admittedWriterScope,
        async (tapPage, _point, { beforeContact }) => {
          elements[0] = createResult(exactPath, {
            left: 110,
            onCommit: () => { committed += 1; },
          });
          const contactPoint = await beforeContact();
          await tapWebViewPoint(tapPage, contactPoint);
        },
      );

      assert.equal(committed, 1);
      assert.deepEqual(sent, [
        { type: "touchStart", x: 134 },
        { type: "touchEnd", x: null },
      ]);
    },
  );
});

test("exact Create capture guard blocks a different target after admission", async () => {
  let wrongCommits = 0;
  const exact = createResult(exactPath);
  const wrong = createResult("notes/wrong.md", { onCommit: () => { wrongCommits += 1; } });
  const elements = [exact];
  await withCreateDom(elements, () => elements[0], async () => {
    const page = {
      async call(fn, ...args) { return fn(...args); },
      async send(_method, params) {
        if (params.type === "touchStart") elements[0] = wrong;
        if (params.type === "touchEnd") wrong.emitClick();
      },
    };

    await assert.rejects(
      clickExactCreateDocument(page, exactPath, admittedWriterScope),
      /did not produce a DOM click.*"blocked":true/,
    );
    assert.equal(wrongCommits, 0);
  });
});

test("exact Create capture guard blocks writer scope drift after admission", async () => {
  let commits = 0;
  const exact = createResult(exactPath, { onCommit: () => { commits += 1; } });
  await withCreateDom([exact], exact, async () => {
    let scopeNonce = "7";
    globalThis.document.querySelector = () => ({
      getAttribute(name) {
        if (name === "data-deve-sync-status") return "ready";
        if (name === "data-deve-repo-id") return "repo-1";
        if (name === "data-deve-scope-nonce") return scopeNonce;
        return null;
      },
    });
    const page = {
      async call(fn, ...args) { return fn(...args); },
      async send(_method, params) {
        if (params.type === "touchStart") scopeNonce = "8";
        if (params.type === "touchEnd") exact.emitClick();
      },
    };

    await assert.rejects(
      clickExactCreateDocument(page, exactPath, admittedWriterScope),
      /did not produce a DOM click.*"blocked":true/,
    );
    assert.equal(commits, 0);
  });
});

test("exact Create cleans a committed-unknown arm and release observation", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    let loseArmResponse = true;
    const armLostPage = {
      async call(fn, ...args) {
        const result = fn(...args);
        if (fn === armExactCreateDocumentClickObservation && loseArmResponse) {
          loseArmResponse = false;
          throw new Error("arm response lost");
        }
        return result;
      },
    };
    await assert.rejects(
      clickExactCreateDocument(
        armLostPage,
        exactPath,
        admittedWriterScope,
        async (_page, _point, { beforeContact }) => beforeContact(),
      ),
      /android_document_create_native_touch_failed/,
    );
    assert.equal(globalThis.__deveAndroidCreatePointerObservation, undefined);
    for (const phase of ["touchstart", "touchend", "pointerdown", "pointerup"]) {
      assert.equal(globalThis.document.listenerCount(phase), 0);
    }

    const releaseLostPage = {
      async call(fn, ...args) { return fn(...args); },
      async send(_method, params) {
        if (params.type !== "touchEnd") return;
        exact.emitClick();
        throw new Error("release response lost");
      },
    };
    await assert.rejects(
      clickExactCreateDocument(releaseLostPage, exactPath, admittedWriterScope),
      /android_document_create_native_touch_failed/,
    );
    assert.equal(globalThis.__deveAndroidCreatePointerObservation, undefined);
  });
});

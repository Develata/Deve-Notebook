import assert from "node:assert/strict";
import test from "node:test";

import {
  armExactCreateDocumentClickObservation,
  clickExactCreateDocument,
  consumeExactCreateDocumentClickObservation,
  consumeExactCreateDocumentClickObservationByPath,
  readExactCreateDocumentPointer,
} from "./lib/android-document-create-flow.mjs";
import { tapWebViewPoint } from "./lib/android-webview-pointer.mjs";

const exactPath = "notes/exact.md";

function withCreateDom(elements, hit, run) {
  const originals = {
    document: globalThis.document,
    getComputedStyle: globalThis.getComputedStyle,
    window: globalThis.window,
    requestAnimationFrame: globalThis.requestAnimationFrame,
    observation: globalThis.__deveAndroidCreatePointerObservation,
  };
  const listeners = new Set();
  globalThis.getComputedStyle = () => ({ display: "block", visibility: "visible" });
  globalThis.window = { innerWidth: 400, innerHeight: 800 };
  globalThis.requestAnimationFrame = (callback) => setImmediate(callback);
  globalThis.document = {
    querySelector: () => null,
    querySelectorAll: () => elements,
    elementFromPoint: (...point) => typeof hit === "function" ? hit(...point) : hit,
    addEventListener: (type, listener) => {
      if (type === "click") listeners.add(listener);
    },
    removeEventListener: (type, listener) => {
      if (type === "click") listeners.delete(listener);
    },
    emitClick(target) {
      const event = {
        target,
        defaultPrevented: false,
        immediatePropagationStopped: false,
        preventDefault() { this.defaultPrevented = true; },
        stopImmediatePropagation() { this.immediatePropagationStopped = true; },
      };
      for (const listener of [...listeners]) {
        listeners.delete(listener);
        listener(event);
        if (event.immediatePropagationStopped) break;
      }
      // Product Create is an event listener, not a browser default action.
      // Only propagation blocking can prevent the wrong typed intent.
      if (!event.immediatePropagationStopped) target.commitClick();
      return event;
    },
  };
  return Promise.resolve(run()).finally(() => {
    for (const [name, value] of Object.entries(originals)) {
      const target = name === "observation" ? "__deveAndroidCreatePointerObservation" : name;
      if (value === undefined) delete globalThis[target];
      else globalThis[target] = value;
    }
  });
}

function createResult(target, { left = 10, onCommit = () => {} } = {}) {
  let currentLeft = left;
  const element = {
    getAttribute: (name) => name === "data-deve-search-result-create-target" ? target : null,
    getBoundingClientRect: () => ({
      left: currentLeft,
      top: 20,
      right: currentLeft + 100,
      bottom: 64,
      width: 100,
      height: 44,
    }),
    contains: () => false,
    closest: () => element,
    commitClick: onCommit,
    emitClick: () => globalThis.document.emitClick(element),
    getLeft: () => currentLeft,
    setLeft: (nextLeft) => { currentLeft = nextLeft; },
  };
  return element;
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
      assert.equal(fn, consumeExactCreateDocumentClickObservation);
      assert.equal(path, 7);
      return { kind: "observed", clicked: true, clickState: null };
    },
  };
  const taps = [];
  await clickExactCreateDocument(page, exactPath, async (tapPage, point, { beforeContact }) => {
    taps.push({ tapPage, point, contactPoint: await beforeContact() });
  });
  assert.deepEqual(taps, [{
    tapPage: page,
    point: { x: 17, y: 23 },
    contactPoint: { x: 17, y: 23 },
  }]);

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
      async (_tapPage, _point, { beforeContact }) => beforeContact(),
    ),
    /changed before native touch contact/,
  );
});

test("exact Create production wiring emits the complete CDP pointer gesture", async () => {
  const exact = createResult(exactPath);
  await withCreateDom([exact], exact, async () => {
    const sent = [];
    const page = {
      async call(fn, ...args) { return fn(...args); },
      async send(method, params) {
        sent.push({ method, params });
        if (params.type === "touchEnd") exact.emitClick();
      },
    };

    const observation = await clickExactCreateDocument(page, exactPath);

    assert.equal(observation.clicked, true);
    assert.deepEqual(sent.map(({ params }) => params.type), [
      "touchStart",
      "touchEnd",
    ]);
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
      clickExactCreateDocument(page, exactPath),
      /did not produce a DOM click.*"blocked":true/,
    );
    assert.equal(wrongCommits, 0);
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
        async (_page, _point, { beforeContact }) => beforeContact(),
      ),
      /arm response lost; unconfirmed_arm_cleanup=.*"kind":"observed"/,
    );
    assert.equal(globalThis.__deveAndroidCreatePointerObservation, undefined);

    const releaseLostPage = {
      async call(fn, ...args) { return fn(...args); },
      async send(_method, params) {
        if (params.type !== "touchEnd") return;
        exact.emitClick();
        throw new Error("release response lost");
      },
    };
    await assert.rejects(
      clickExactCreateDocument(releaseLostPage, exactPath),
      /release response lost; touch_cancel=confirmed; click_observation=.*"clicked":true/,
    );
    assert.equal(globalThis.__deveAndroidCreatePointerObservation, undefined);
  });
});

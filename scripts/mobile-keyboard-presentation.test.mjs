import assert from "node:assert/strict";
import test from "node:test";

import { proveAndroidImeBackPriority } from "./lib/mobile-ime-back-proof.mjs";
import { proveSameBreakpointKeyboardPresentation } from "./lib/mobile-keyboard-presentation.mjs";
import { sameEditorLoadSession } from "./lib/mobile-webview-interaction.mjs";

const hiddenBaseline = () => ({
  hostId: 7,
  openRequestId: "11",
  docId: "doc-1",
  repoId: "repo-1",
  scopeNonce: "9",
  presentationGeneration: 3,
  presentationEpoch: 4,
  selectionIdentity: { from: 5, to: 5, rangeCount: 1 },
  viewportWidth: 392,
  innerHeight: 872,
  visualViewportHeight: 872,
  activeEditor: true,
  bridgeReady: true,
  activeHostMatchesVisible: true,
  keyboardPresentation: "hidden",
  keyboardOffset: 0,
  nativeKeyboardOverlay: 0,
});

const focusedEditorResult = {
  tag: "DIV",
  className: "cm-content",
  contentEditable: "true",
  activeEditor: true,
  identityMatched: true,
  visualViewportHeight: 872,
};

function keyboardProofPage(baseline, presented) {
  const focused = { ...baseline, activeEditor: true };
  const calls = [baseline, { x: 24, y: 32 }, focusedEditorResult, focused, presented];
  return { async call() { return calls.shift(); } };
}

test("keyboard presentation timeout reports viewport and native fallback observations", async () => {
  const baseline = hiddenBaseline();
  const last = { ...baseline, activeEditor: false, visualViewportHeight: 510 };
  const page = keyboardProofPage(baseline, last);
  let waits = 0;
  const waitUntil = async (_label, predicate) => {
    waits += 1;
    const value = await predicate();
    if (waits === 1) return value;
    throw new Error("synthetic presentation timeout");
  };

  await assert.rejects(
    proveSameBreakpointKeyboardPresentation(page, {
      waitUntil,
      activateKeyboard: async () => {},
    }),
    /android_keyboard_presentation_observation=.*"activeEditor":false.*"visualViewportHeight":510.*"keyboardPresentation":"hidden"/,
  );
});

test("keyboard presentation accepts current native inset when visual viewport is unchanged", async () => {
  const baseline = hiddenBaseline();
  const nativeFallback = {
    ...baseline,
    keyboardPresentation: "native-insets",
    keyboardOffset: 338,
    nativeKeyboardOverlay: 338,
    toolbarBottomGap: 338,
    presentationEpoch: 5,
  };
  const result = await proveSameBreakpointKeyboardPresentation(
    keyboardProofPage(baseline, nativeFallback),
    {
      waitUntil: async (_label, predicate) => {
        const value = await predicate();
        assert.ok(value);
        return value;
      },
      activateKeyboard: async () => {},
    },
  );
  assert.equal(result.presented.keyboardMode, "native-insets");
  assert.equal(sameEditorLoadSession(result.focused, result.presented), true);
});

test("keyboard presentation keeps visual viewport primary when native inset is also available", async () => {
  const baseline = hiddenBaseline();
  const visualViewport = {
    ...baseline,
    visualViewportHeight: 510,
    keyboardPresentation: "visual-viewport",
    keyboardOffset: 362,
    nativeKeyboardOverlay: 0,
    toolbarBottomGap: 0,
    presentationEpoch: 5,
  };
  const result = await proveSameBreakpointKeyboardPresentation(
    keyboardProofPage(baseline, visualViewport),
    {
      waitUntil: async (_label, predicate) => {
        const value = await predicate();
        assert.ok(value);
        return value;
      },
      activateKeyboard: async () => {},
    },
  );
  assert.equal(result.presented.keyboardMode, "visual-viewport");
  assert.equal(result.presented.nativeKeyboardOverlay, 0);
});

test("adjustResize viewport shrink is visible with zero additional overlay offset", async () => {
  const baseline = hiddenBaseline();
  const resized = {
    ...baseline,
    innerHeight: 510,
    visualViewportHeight: 510,
    keyboardPresentation: "visual-viewport",
    keyboardOffset: 0,
    nativeKeyboardOverlay: 0,
    toolbarBottomGap: 0,
    presentationEpoch: 5,
  };
  const result = await proveSameBreakpointKeyboardPresentation(
    keyboardProofPage(baseline, resized),
    {
      waitUntil: async (_label, predicate) => {
        const value = await predicate();
        assert.ok(value);
        return value;
      },
      activateKeyboard: async () => {},
    },
  );
  assert.equal(result.presented.keyboardMode, "visual-viewport");
  assert.equal(result.presented.keyboardOffset, 0);
});

test("native fallback is rejected after viewport already shrank from hidden baseline", async () => {
  const baseline = hiddenBaseline();
  const staleNative = {
    ...baseline,
    innerHeight: 510,
    visualViewportHeight: 510,
    keyboardPresentation: "native-insets",
    keyboardOffset: 338,
    nativeKeyboardOverlay: 338,
    toolbarBottomGap: 338,
    presentationEpoch: 5,
  };
  let waits = 0;
  await assert.rejects(
    proveSameBreakpointKeyboardPresentation(keyboardProofPage(baseline, staleNative), {
      waitUntil: async (_label, predicate) => {
        waits += 1;
        const value = await predicate();
        if (waits === 1) return value;
        throw new Error("synthetic presentation timeout");
      },
      activateKeyboard: async () => {},
    }),
    /synthetic presentation timeout/,
  );
});

test("native fallback is rejected after even a small viewport shrink", async () => {
  const baseline = hiddenBaseline();
  const staleNative = {
    ...baseline,
    innerHeight: 870,
    visualViewportHeight: 870,
    keyboardPresentation: "native-insets",
    keyboardOffset: 338,
    nativeKeyboardOverlay: 338,
    toolbarBottomGap: 338,
    presentationEpoch: 5,
  };
  let waits = 0;
  await assert.rejects(
    proveSameBreakpointKeyboardPresentation(keyboardProofPage(baseline, staleNative), {
      waitUntil: async (_label, predicate) => {
        waits += 1;
        const value = await predicate();
        if (waits === 1) return value;
        throw new Error("synthetic presentation timeout");
      },
      activateKeyboard: async () => {},
    }),
    /synthetic presentation timeout/,
  );
});

test("keyboard proof rejects a non-hidden baseline", async () => {
  const invalidBaseline = {
    ...hiddenBaseline(),
    keyboardPresentation: "native-insets",
    keyboardOffset: 338,
    nativeKeyboardOverlay: 338,
  };
  const page = keyboardProofPage(invalidBaseline, invalidBaseline);
  await assert.rejects(
    proveSameBreakpointKeyboardPresentation(page, {
      waitUntil: async (label, predicate) => {
        const value = await predicate();
        if (label === "Android keyboard hidden baseline") {
          throw new Error(`baseline rejected: ${value}`);
        }
        return value;
      },
      activateKeyboard: async () => {},
    }),
    /baseline rejected: null/,
  );
});

test("Android Back waits through transient selection drift and retap may move the caret", async () => {
  const base = {
    hostId: 7,
    openRequestId: "11",
    point: { x: 24, y: 32 },
    activeEditor: true,
    bridgeReady: true,
    activeHostMatchesVisible: true,
    docId: "doc-1",
    repoId: "repo-1",
    scopeNonce: "9",
    presentationGeneration: 3,
    presentationEpoch: 5,
    selectionIdentity: { from: 5, to: 5, rangeCount: 1 },
    innerHeight: 800,
    visualViewportHeight: 800,
  };
  const observations = [
    { ...base, keyboardPresentation: "native-insets", keyboardOffset: 300, nativeKeyboardOverlay: 300 },
    {
      ...base,
      selectionIdentity: { from: 6, to: 6, rangeCount: 1 },
      keyboardPresentation: "hidden",
      keyboardOffset: 0,
      nativeKeyboardOverlay: 0,
    },
    { ...base, keyboardPresentation: "hidden", keyboardOffset: 0, nativeKeyboardOverlay: 0 },
    {
      ...base,
      selectionIdentity: { from: 8, to: 8, rangeCount: 1 },
      keyboardPresentation: "native-insets",
      keyboardOffset: 300,
      nativeKeyboardOverlay: 300,
    },
  ];
  const order = [];
  const waitUntil = async (label, predicate) => {
    order.push(label);
    for (let attempt = 0; attempt < 4; attempt += 1) {
      const value = await predicate();
      if (value) return value;
    }
    assert.fail(`${label} must succeed`);
  };

  const proof = await proveAndroidImeBackPriority({}, {
    waitUntil,
    platformBack: async () => order.push("platform-back"),
    activateKeyboard: async (point) => {
      assert.deepEqual(point, base.point);
      order.push("activate-keyboard");
    },
    observeEditor: async () => observations.shift(),
  });

  assert.equal(sameEditorLoadSession(proof.before, proof.hidden), true);
  assert.equal(sameEditorLoadSession(proof.hidden, proof.reopened), true);
  assert.deepEqual(proof.hidden.selectionIdentity, base.selectionIdentity);
  assert.deepEqual(proof.reopened.selectionIdentity, { from: 8, to: 8, rangeCount: 1 });
  assert.deepEqual(order, [
    "Android IME-visible editor session",
    "platform-back",
    "Android IME-only back dismissal",
    "activate-keyboard",
    "Android IME reopened on the same editor",
  ]);
});

test("Android Back rejects persistent selection drift at the proof boundary", async () => {
  const base = {
    ...hiddenBaseline(),
    point: { x: 24, y: 32 },
    keyboardPresentation: "native-insets",
    keyboardOffset: 300,
    nativeKeyboardOverlay: 300,
  };
  const drifted = {
    ...base,
    keyboardPresentation: "hidden",
    keyboardOffset: 0,
    nativeKeyboardOverlay: 0,
    selectionIdentity: { from: 6, to: 6, rangeCount: 1 },
  };
  const observations = [base, drifted, drifted, drifted, drifted];

  await assert.rejects(
    proveAndroidImeBackPriority({}, {
      waitUntil: async (label, predicate) => {
        for (let attempt = 0; attempt < 4; attempt += 1) {
          const value = await predicate();
          if (value) return value;
        }
        throw new Error(`synthetic timeout during ${label}`);
      },
      platformBack: async () => {},
      activateKeyboard: async () => {},
      observeEditor: async () => observations.shift() ?? drifted,
    }),
    /android_ime_back_observation=.*"selectionIdentity":\{"from":6,"to":6,"rangeCount":1\}/,
  );
});

test("keyboard presentation rejects selection drift without a user touch", async () => {
  const baseline = hiddenBaseline();
  const drifted = {
    ...baseline,
    keyboardPresentation: "native-insets",
    keyboardOffset: 338,
    nativeKeyboardOverlay: 338,
    toolbarBottomGap: 338,
    presentationEpoch: 5,
    selectionIdentity: { from: 6, to: 6, rangeCount: 1 },
  };

  await assert.rejects(
    proveSameBreakpointKeyboardPresentation(keyboardProofPage(baseline, drifted), {
      waitUntil: async (_label, predicate) => {
        const value = await predicate();
        assert.ok(value);
        return value;
      },
      activateKeyboard: async () => {},
    }),
    /android_keyboard_presentation_observation=.*"selectionIdentity":\{"from":6,"to":6,"rangeCount":1\}/,
  );
});

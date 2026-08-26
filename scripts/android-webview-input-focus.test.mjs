import assert from "node:assert/strict";
import test from "node:test";

import { waitForCurrentWebViewInputFocus } from "./lib/android-webview-input-focus.mjs";

async function runInputFocusSamples(samples) {
  const originalNow = Date.now;
  const observedErrors = [];
  let now = 0;
  Date.now = () => now;
  const page = {
    calls: 0,
    async call() {
      const sample = samples[Math.min(this.calls, samples.length - 1)];
      this.calls += 1;
      if (sample instanceof Error) throw sample;
      return sample;
    },
  };
  const waitUntil = async (_label, predicate) => {
    for (let index = 0; index < 10; index += 1) {
      try {
        if (await predicate()) return true;
      } catch (error) {
        observedErrors.push(error.message);
      }
      now += 125;
    }
    throw new Error("synthetic focus timeout");
  };
  try {
    await waitForCurrentWebViewInputFocus(page, waitUntil);
    return { calls: page.calls, observedErrors };
  } finally {
    Date.now = originalNow;
  }
}

test("Android WebView native input waits for continuous visible current focus", async () => {
  const result = await runInputFocusSamples([
    { documentTimeOrigin: 1, visible: true, focused: false, mobile: true },
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 1, visible: false, focused: true, mobile: true },
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
  ]);
  assert.deepEqual(result, { calls: 7, observedErrors: [] });
});

test("Android WebView input focus settlement restarts after document replacement", async () => {
  const result = await runInputFocusSamples([
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 2, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 2, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 2, visible: true, focused: true, mobile: true },
  ]);
  assert.deepEqual(result, { calls: 5, observedErrors: [] });
});

test("Android WebView input focus settlement restarts after a redacted sample failure", async () => {
  const secretSentinel = "secret=/private/device/path";
  const result = await runInputFocusSamples([
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
    new Error(secretSentinel),
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
    { documentTimeOrigin: 1, visible: true, focused: true, mobile: true },
  ]);
  assert.deepEqual(result, {
    calls: 6,
    observedErrors: ["android_webview_input_focus_sample_failed"],
  });
  assert.doesNotMatch(JSON.stringify(result), new RegExp(secretSentinel));
});

test("Android WebView input focus timeout keeps the fixed label and caller budget", async () => {
  const page = {
    async call() {
      return { documentTimeOrigin: 1, visible: false, focused: false, mobile: true };
    },
  };
  let observedLabel;
  let observedTimeout;
  const waitUntil = async (label, predicate, timeout) => {
    observedLabel = label;
    observedTimeout = timeout;
    assert.equal(await predicate(), false);
    throw new Error("synthetic focus timeout");
  };
  await assert.rejects(
    waitForCurrentWebViewInputFocus(page, waitUntil, 1234),
    /synthetic focus timeout/,
  );
  assert.equal(observedLabel, "current Android WebView input focus settlement");
  assert.equal(observedTimeout, 1234);
});

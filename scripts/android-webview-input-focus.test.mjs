import assert from "node:assert/strict";
import test from "node:test";

import {
  classifyAndroidNativeInputTarget,
  classifyAndroidWindowFocused,
  createAndroidWebViewInputTargetGate,
  waitForCurrentAndroidWebViewInputTarget,
  waitForCurrentWebViewInputFocus,
} from "./lib/android-webview-input-focus.mjs";

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

test("Android current window classifier admits only the exact focused app component", () => {
  const appId = "dev.deve.notebook.mobile";
  assert.equal(classifyAndroidWindowFocused(
    "  mCurrentFocus=Window{b6877d0 u0 dev.deve.notebook.mobile/dev.deve.notebook.mobile.MainActivity}",
    appId,
  ), "focused");
  assert.equal(classifyAndroidWindowFocused(
    "  mCurrentFocus=Window{b6877d0 u0 com.android.systemui/.statusbar.StatusBar}",
    appId,
  ), "not-focused");
  assert.equal(classifyAndroidWindowFocused("  mCurrentFocus=null", appId), "unavailable");
  assert.equal(classifyAndroidWindowFocused(
    "  mCurrentFocus=Window{b6877d0 u0 dev.deve.notebook.mobile.preview/.MainActivity}",
    appId,
  ), "unavailable");
  assert.equal(classifyAndroidWindowFocused([
    "mCurrentFocus=Window{a u0 dev.deve.notebook.mobile/.MainActivity}",
    "mCurrentFocus=Window{b u0 com.android.systemui/.statusbar.StatusBar}",
  ].join("\n"), appId), "unavailable");
});

test("Android native input classifier requires resumed Activity and exact current window", () => {
  const appId = "dev.deve.notebook.mobile";
  const resumed =
    "topResumedActivity=ActivityRecord{8ea2038 u0 dev.deve.notebook.mobile/.MainActivity t7061}";
  const focused =
    "mCurrentFocus=Window{b6877d0 u0 dev.deve.notebook.mobile/dev.deve.notebook.mobile.MainActivity}";
  assert.equal(classifyAndroidNativeInputTarget(resumed, focused, appId), "ready");
  assert.equal(classifyAndroidNativeInputTarget(
    resumed,
    "mCurrentFocus=Window{b6877d0 u0 com.android.systemui/.statusbar.StatusBar}",
    appId,
  ), "not-ready");
  assert.equal(classifyAndroidNativeInputTarget(resumed, "mCurrentFocus=null", appId), "unavailable");
});

test("Android native input gate samples resumed Activity and current window together", async () => {
  const originalNow = Date.now;
  let now = 0;
  Date.now = () => now;
  const appId = "dev.deve.notebook.mobile";
  const adbCalls = [];
  const adbOutput = (...args) => {
    adbCalls.push(args);
    if (args.join(" ") === "shell dumpsys activity activities") {
      return `topResumedActivity=ActivityRecord{8ea2038 u0 ${appId}/.MainActivity t7061}`;
    }
    if (args.join(" ") === "shell dumpsys window") {
      return `mCurrentFocus=Window{b6877d0 u0 ${appId}/${appId}.MainActivity}`;
    }
    throw new Error("unexpected ADB probe");
  };
  const page = {
    async call() {
      return { documentTimeOrigin: 1, visible: true, focused: true, mobile: true };
    },
  };
  const waitUntil = async (label, predicate) => {
    assert.equal(label, "current Android native input target settlement");
    for (let index = 0; index < 4; index += 1) {
      if (await predicate()) return true;
      now += 125;
    }
    throw new Error("synthetic native input target timeout");
  };
  try {
    await createAndroidWebViewInputTargetGate(adbOutput, appId, () => {
      throw new Error("foreground reentry must not run for an admitted target");
    })(page, waitUntil);
    assert.equal(adbCalls.length, 6);
    assert.deepEqual(adbCalls.slice(0, 2), [
      ["shell", "dumpsys", "activity", "activities"],
      ["shell", "dumpsys", "window"],
    ]);
  } finally {
    Date.now = originalNow;
  }
});

test("Android native input gate rejects a missing ADB probe with a fixed category", () => {
  assert.throws(
    () => createAndroidWebViewInputTargetGate(null, "dev.deve.notebook.mobile"),
    /android_native_input_target_adb_probe_missing/,
  );
  assert.throws(
    () => createAndroidWebViewInputTargetGate(() => "", Symbol("invalid"), () => {}),
    /android_native_input_target_gate_config_invalid/,
  );
});

test("Android native input gate allows one PID-stable launcher reentry", async () => {
  const originalNow = Date.now;
  let now = 0;
  let foregrounded = false;
  let reentryCount = 0;
  Date.now = () => now;
  const appId = "dev.deve.notebook.mobile";
  const adbOutput = (...args) => {
    const command = args.join(" ");
    if (command === `shell pidof ${appId}`) return "1234\n";
    if (command === "shell dumpsys activity activities") {
      const packageName = foregrounded ? appId : "com.android.launcher3";
      return `topResumedActivity=ActivityRecord{a u0 ${packageName}/.MainActivity t1}`;
    }
    if (command === "shell dumpsys window") {
      const packageName = foregrounded ? appId : "com.android.launcher3";
      return `mCurrentFocus=Window{b u0 ${packageName}/${packageName}.MainActivity}`;
    }
    throw new Error("unexpected ADB probe");
  };
  const adbCommand = (...args) => {
    assert.deepEqual(args, [
      "shell", "monkey", "-p", appId,
      "-c", "android.intent.category.LAUNCHER", "1",
    ]);
    reentryCount += 1;
    foregrounded = true;
  };
  const page = {
    async call() {
      return { documentTimeOrigin: 1, visible: true, focused: true, mobile: true };
    },
  };
  let waitCall = 0;
  const waitUntil = async (_label, predicate) => {
    waitCall += 1;
    const samples = waitCall === 1 ? 1 : 4;
    for (let index = 0; index < samples; index += 1) {
      if (await predicate()) return true;
      now += 125;
    }
    throw new Error("synthetic native input target timeout");
  };
  try {
    await createAndroidWebViewInputTargetGate(adbOutput, appId, adbCommand, {
      passiveTimeoutMs: 1,
      reentryTimeoutMs: 1000,
    })(page, waitUntil);
    assert.equal(reentryCount, 1);
    assert.equal(waitCall, 2);
  } finally {
    Date.now = originalNow;
  }
});

test("Android native input reentry failure reports only fixed target categories", async () => {
  const appId = "dev.deve.notebook.mobile";
  const secretSentinel = "secret=/private/input/window";
  const adbOutput = (...args) => {
    const command = args.join(" ");
    if (command === `shell pidof ${appId}`) return "1234\n";
    if (command === "shell dumpsys activity activities") {
      return "topResumedActivity=ActivityRecord{a u0 com.android.launcher3/.MainActivity t1}";
    }
    if (command === "shell dumpsys window") {
      return "mCurrentFocus=Window{b u0 com.android.launcher3/.MainActivity}";
    }
    throw new Error(secretSentinel);
  };
  const page = {
    async call() {
      return { documentTimeOrigin: 1, visible: true, focused: true, mobile: true };
    },
  };
  const waitUntil = async (_label, predicate) => {
    await predicate();
    throw new Error(secretSentinel);
  };
  await assert.rejects(
    createAndroidWebViewInputTargetGate(adbOutput, appId, () => {}, {
      passiveTimeoutMs: 1,
      reentryTimeoutMs: 1,
    })(page, waitUntil),
    (error) => {
      assert.match(
        error.message,
        /android_native_input_target_reentry_failed; android_native_input_target_settlement_failed; last=.*"nativeTargetState":"not-ready"/,
      );
      assert.doesNotMatch(error.message, /secret|private|input\/window/);
      return true;
    },
  );
});

test("Android native input reentry rejects PID replacement", async () => {
  const originalNow = Date.now;
  let now = 0;
  let foregrounded = false;
  let pidReads = 0;
  let waitRound = 0;
  Date.now = () => now;
  const appId = "dev.deve.notebook.mobile";
  const adbOutput = (...args) => {
    const command = args.join(" ");
    if (command === `shell pidof ${appId}`) {
      pidReads += 1;
      return pidReads === 1 ? "1234\n" : "5678\n";
    }
    const packageName = foregrounded ? appId : "com.android.launcher3";
    if (command === "shell dumpsys activity activities") {
      return `topResumedActivity=ActivityRecord{a u0 ${packageName}/.MainActivity t1}`;
    }
    if (command === "shell dumpsys window") {
      return `mCurrentFocus=Window{b u0 ${packageName}/${packageName}.MainActivity}`;
    }
    throw new Error("unexpected ADB probe");
  };
  const page = {
    async call() {
      return { documentTimeOrigin: 1, visible: true, focused: true, mobile: true };
    },
  };
  const waitUntil = async (_label, predicate) => {
    waitRound += 1;
    if (waitRound === 1) {
      await predicate();
      throw new Error("synthetic passive timeout");
    }
    assert.equal(await predicate(), false);
    now += 250;
    assert.equal(await predicate(), true);
    return true;
  };
  try {
    await assert.rejects(
      createAndroidWebViewInputTargetGate(
        adbOutput,
        appId,
        () => { foregrounded = true; },
        { passiveTimeoutMs: 1, reentryTimeoutMs: 1000 },
      )(page, waitUntil),
      /android_native_input_target_reentry_pid_unstable/,
    );
  } finally {
    Date.now = originalNow;
  }
});

test("Android native input target resets settlement until Activity and window are current", async () => {
  const originalNow = Date.now;
  let now = 0;
  let nativeTargetSamples = 0;
  Date.now = () => now;
  const nativeTargetStates = ["not-ready", "ready", "ready", "ready"];
  const page = {
    calls: 0,
    async call() {
      this.calls += 1;
      return { documentTimeOrigin: 1, visible: true, focused: true, mobile: true };
    },
  };
  const waitUntil = async (label, predicate) => {
    assert.equal(label, "current Android native input target settlement");
    for (let index = 0; index < 8; index += 1) {
      if (await predicate()) return true;
      now += 125;
    }
    throw new Error("synthetic native input target timeout");
  };
  try {
    await waitForCurrentAndroidWebViewInputTarget(page, waitUntil, async () => {
      const state = nativeTargetStates[
        Math.min(nativeTargetSamples, nativeTargetStates.length - 1)
      ];
      nativeTargetSamples += 1;
      return state;
    });
    assert.equal(page.calls, 4);
    assert.equal(nativeTargetSamples, 4);
  } finally {
    Date.now = originalNow;
  }
});

test("Android native input target redacts platform probe failures", async () => {
  const page = {
    async call() {
      return { documentTimeOrigin: 1, visible: true, focused: true, mobile: true };
    },
  };
  const observed = [];
  const waitUntil = async (_label, predicate) => {
    try {
      await predicate();
    } catch (error) {
      observed.push(error.message);
    }
    throw new Error("synthetic native input target timeout");
  };
  await assert.rejects(
    waitForCurrentAndroidWebViewInputTarget(page, waitUntil, () => {
      throw new Error("secret=/private/window/path");
    }),
    /android_native_input_target_settlement_failed/,
  );
  assert.deepEqual(observed, ["android_native_input_target_sample_failed"]);
  assert.doesNotMatch(JSON.stringify(observed), /secret|private|window\/path/);
});

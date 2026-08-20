import assert from "node:assert/strict";
import test from "node:test";

import {
  classifyAndroidActivityResumed,
  createAndroidLifecycleHarness,
  requireAndroidRootBackStablePid,
} from "./lib/android-lifecycle-harness.mjs";

const appId = "dev.deve.notebook.mobile";

for (const [label, sample] of [
  [
    "AOSP mResumedActivity",
    "  mResumedActivity: ActivityRecord{abc u0 dev.deve.notebook.mobile/.MainActivity t12}",
  ],
  [
    "HyperOS topResumedActivity",
    "      topResumedActivity=ActivityRecord{abc u0 dev.deve.notebook.mobile/.MainActivity t12}",
  ],
  [
    "HyperOS ResumedActivity",
    "  ResumedActivity: ActivityRecord{abc u0 dev.deve.notebook.mobile/.MainActivity t12}",
  ],
]) {
  test(`Android resumed classifier accepts ${label}`, () => {
    assert.equal(classifyAndroidActivityResumed(sample, appId), "resumed");
  });
}

test("Android resumed classifier recognizes a different foreground task as background", () => {
  const sample = [
    "topResumedActivity=ActivityRecord{abc u0 com.miui.home/.launcher.Launcher t2}",
    "ResumedActivity: ActivityRecord{abc u0 com.miui.home/.launcher.Launcher t2}",
  ].join("\n");
  assert.equal(classifyAndroidActivityResumed(sample, appId), "not-resumed");
});

test("Android resumed classifier fails closed on package prefix collisions", () => {
  for (const packageName of [
    "dev.deve.notebook.mobile.evil",
    "dev.deve.notebook",
  ]) {
    const sample = `mResumedActivity: ActivityRecord{abc u0 ${packageName}/.MainActivity t2}`;
    assert.equal(classifyAndroidActivityResumed(sample, appId), "unavailable");
  }
});

test("Android resumed classifier fails closed when dumpsys exposes no admitted key", () => {
  const sample = "FocusedActivity: ActivityRecord{abc u0 dev.deve.notebook.mobile/.MainActivity t2}";
  assert.equal(classifyAndroidActivityResumed(sample, appId), "unavailable");
  assert.equal(classifyAndroidActivityResumed(sample, "invalid-app-id"), "unavailable");
});

test("Android resumed classifier fails closed on a malformed admitted record", () => {
  assert.equal(classifyAndroidActivityResumed("mResumedActivity:", appId), "unavailable");
  assert.equal(classifyAndroidActivityResumed("mResumedActivity: null", appId), "unavailable");
  assert.equal(
    classifyAndroidActivityResumed(
      "mResumedActivity: ActivityRecord{abc u0 dev.deve.notebook.mobile/.MainActivity truncated",
      appId,
    ),
    "unavailable",
  );
  assert.equal(
    classifyAndroidActivityResumed([
      "topResumedActivity=ActivityRecord{abc u0 dev.deve.notebook.mobile/.MainActivity t12}",
      "ResumedActivity: null",
    ].join("\n"), appId),
    "unavailable",
  );
});

test("Android resumed classifier fails closed when admitted keys conflict", () => {
  const sample = [
    "topResumedActivity=ActivityRecord{abc u0 dev.deve.notebook.mobile/.MainActivity t12}",
    "ResumedActivity: ActivityRecord{def u0 com.miui.home/.launcher.Launcher t2}",
  ].join("\n");
  assert.equal(classifyAndroidActivityResumed(sample, appId), "unavailable");
});

function readyRootReentryObservation(epoch = 8) {
  return {
    state: {
      backend_running: true,
      service_state: "endpoint_session_ready",
    },
    projection: {
      syncStatus: "handshaking-repo",
      repoIdRaw: "",
      loginVisible: false,
      bootstrapSessionBound: true,
      nativeSessionInstalled: true,
    },
    presentation: { generation: 1, epoch },
  };
}

test("root Back reentry retries one bounded stalled readonly sample", async () => {
  const harness = createAndroidLifecycleHarness({
    timeoutMs: 1_000,
    rootReentrySampleTimeoutMs: 10,
    adb: "unused",
    serial: "unused",
  });
  let attempts = 0;
  const result = await harness.waitForAndroidRootReentry(() => {
    attempts += 1;
    return attempts === 1 ? new Promise(() => {}) : readyRootReentryObservation();
  }, { generation: 1, epoch: 7 });
  assert.equal(result, true);
  assert.equal(attempts, 2);
});

test("root Back reentry sampling failure exposes only fixed bounded diagnostics", async () => {
  const harness = createAndroidLifecycleHarness({
    timeoutMs: 300,
    rootReentrySampleTimeoutMs: 10,
    adb: "unused",
    serial: "unused",
  });
  const startedAt = Date.now();
  await assert.rejects(
    harness.waitForAndroidRootReentry(
      () => Promise.reject(new Error("secret=/private/runner/path")),
      { generation: 1, epoch: 7 },
    ),
    (error) => {
      assert.match(error.message, /android_root_reentry_sample_failed/);
      assert.match(error.message, /sampleFailures=[1-9][0-9]*/);
      assert.doesNotMatch(error.message, /secret|private|runner|path/);
      return true;
    },
  );
  assert.ok(Date.now() - startedAt < 1_000, "sample retries must not extend the total deadline");
});

test("root Back surface observation failure exposes only a fixed category", async () => {
  const harness = createAndroidLifecycleHarness({
    timeoutMs: 1_000,
    adb: "unused",
    serial: "unused",
  });
  const observation = await harness.readAndroidUiBackSurfaceObservation({
    call: () => Promise.reject(new Error("secret=/private/runner/path")),
  });
  assert.deepEqual(observation, {
    observationAvailable: false,
    category: "android_ui_back_surface_observation_failed",
  });
  assert.doesNotMatch(JSON.stringify(observation), /secret|private|runner|path/);
});

test("root Back PID replacement fails with a fixed category", () => {
  assert.equal(requireAndroidRootBackStablePid("4431", "4431"), true);
  for (const [before, after] of [["4431", "7788"], ["", "4431"], ["4431", "4431 7788"]]) {
    assert.throws(
      () => requireAndroidRootBackStablePid(before, after),
      (error) => error.message === "android_root_back_pid_unstable",
    );
  }
});

test("root Back proof hides raw ADB failures behind a fixed category", async () => {
  const harness = createAndroidLifecycleHarness({
    timeoutMs: 1_000,
    adb: "secret-private-runner-adb",
    serial: "unused",
  });
  await assert.rejects(
    harness.proveAndroidRootBackBackground(appId, async () => {}),
    (error) => error.message === "android_root_back_proof_failed",
  );
});

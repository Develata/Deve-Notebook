import assert from "node:assert/strict";
import test from "node:test";

import { classifyAndroidActivityResumed } from "./lib/android-lifecycle-harness.mjs";

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

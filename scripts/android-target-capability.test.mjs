import assert from "node:assert/strict";
import test from "node:test";
import {
  buildAndroidTargetFacts,
  evaluateWritableProbeExpectation,
  parseCurrentWebViewProvider,
} from "./lib/android-target-capability.mjs";

test("parses the tuple returned by cmd webviewupdate", () => {
  assert.deepEqual(
    parseCurrentWebViewProvider(
      "Current WebView package (name, version): (com.google.android.webview, 137.0.7151.89)",
    ),
    { packageName: "com.google.android.webview", versionName: "137.0.7151.89" },
  );
});

test("parses dumpsys provider lines without relying on a user agent", () => {
  assert.deepEqual(
    parseCurrentWebViewProvider("Current WebView package is com.android.webview 133.0.6943.121"),
    { packageName: "com.android.webview", versionName: "133.0.6943.121" },
  );
});

test("does not promote a preferred provider when the current provider is null", () => {
  assert.deepEqual(
    parseCurrentWebViewProvider([
      "Current WebView package is null",
      "Preferred WebView package (name, version): (com.google.android.webview, 137.0.7151.89)",
    ].join("\n")),
    { packageName: null, versionName: null },
  );
});

test("support qualification requires both Android 10 and provider 137", () => {
  const qualified = buildAndroidTargetFacts({
    sdkRaw: "29",
    webViewRaw: "Current WebView package (name, version): (com.google.android.webview, 137.0.7151.89)",
    avdName: "Pixel_API_35",
    buildFingerprint: "google/sdk_gphone64_x86_64/test",
    model: "sdk_gphone64_x86_64",
  });
  assert.equal(qualified.supportBaseline, true);
  assert.equal(qualified.webViewProviderMajor, 137);

  assert.equal(buildAndroidTargetFacts({
    sdkRaw: "28",
    webViewRaw: "Current WebView package (name, version): (com.google.android.webview, 137.0.7151.89)",
  }).supportBaseline, false);
  assert.equal(buildAndroidTargetFacts({
    sdkRaw: "35",
    webViewRaw: "Current WebView package (name, version): (com.android.webview, 133.0.6943.121)",
  }).supportBaseline, false);
});

test("probe expectation distinguishes writable proof from honest negative evidence", () => {
  assert.equal(evaluateWritableProbeExpectation(true, { writable: true }), "writable");
  assert.equal(
    evaluateWritableProbeExpectation(false, { writable: false, blocker: "ed25519_unavailable" }),
    "readonly-negative",
  );
  assert.throws(
    () => evaluateWritableProbeExpectation(true, { writable: false, blocker: "ed25519_unavailable" }),
    /failed Ed25519 probe/,
  );
  assert.throws(
    () => evaluateWritableProbeExpectation(false, { writable: true }),
    /unexpectedly passed/,
  );
  assert.throws(
    () => evaluateWritableProbeExpectation(false, {
      writable: false,
      blocker: "capability_probe_failed",
    }),
    /stable unsupported blocker/,
  );
});

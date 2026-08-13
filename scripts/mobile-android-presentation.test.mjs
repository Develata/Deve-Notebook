import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const read = (path) => fs.readFileSync(new URL(path, import.meta.url), "utf8");
const localJourney = read("./smoke-mobile-android-lifecycle.mjs");
const remoteJourney = read("./smoke-mobile-android-remote-browser.mjs");
const presentationProof = read("./lib/android-presentation-proof.mjs");
const nativeDispatcher = read("../apps/mobile/gen/android/app/src/main/java/dev/deve/notebook/mobile/NativePresentationDispatcher.kt");
const webPresentation = read("../apps/web/src/components/mobile_layout/native_presentation.rs");

test("native presentation is re-admitted after same-WebView reload before drawer gestures", () => {
  const localReload = localJourney.indexOf("await reloadWithWebSocketDeliveryGate(page)");
  const localProof = localJourney.indexOf("await proveAndroidDrawerGesturesAfterReload(page");
  const remoteReload = remoteJourney.indexOf("await observeRemoteGeneration(page, observations)");
  const remoteProof = remoteJourney.indexOf("await waitForAcceptedAndroidPresentation(page");
  assert.ok(localReload >= 0 && localReload < localProof);
  assert.ok(remoteReload >= 0 && remoteReload < remoteProof);
  assert.match(presentationProof, /data-deve-native-presentation/);
  assert.match(presentationProof, /data-deve-mobile-drawer="left"/);
  assert.match(presentationProof, /data-deve-mobile-drawer="right"/);
  assert.match(presentationProof, /assert\.equal\(pidAfter, pidBefore/);
  assert.match(localJourney, /nativeDrawerGesturesAfterReload:/);
  assert.match(remoteJourney, /nativeSystemGestureInsetsAcceptedAfterReload: true/);
});

test("current presentation becomes pending before replacement Insets can re-arm gestures", () => {
  const pending = nativeDispatcher.indexOf("scheduleInvalidation(source, webViewGeneration, epoch, 0)");
  const ready = nativeDispatcher.indexOf("schedulePublish(source, generation, epoch, 0)");
  assert.ok(pending >= 0 && ready >= 0 && pending < ready);
  assert.match(nativeDispatcher, /system-gesture-insets-pending/);
  assert.match(nativeDispatcher, /ViewCompat\.setOnApplyWindowInsetsListener\(observer\)/);
  assert.match(nativeDispatcher, /FrameLayout\.LayoutParams\(0, 0\)/);
  assert.match(webPresentation, /Some\("system-gesture-insets-pending"\)/);
  assert.match(webPresentation, /set_system_gesture_insets\.set\(None\)/);
  assert.match(webPresentation, /if order < apply_order\.get\(\)/);
});

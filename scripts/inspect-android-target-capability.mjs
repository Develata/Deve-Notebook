import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { buildAndroidTargetFacts } from "./lib/android-target-capability.mjs";

const facts = buildAndroidTargetFacts({
  sdkRaw: process.env.DEVE_ANDROID_TARGET_SDK_RAW,
  webViewRaw: process.env.DEVE_ANDROID_TARGET_WEBVIEW_RAW,
  avdName: process.env.DEVE_ANDROID_TARGET_AVD_NAME,
  buildFingerprint: process.env.DEVE_ANDROID_TARGET_BUILD_FINGERPRINT,
  model: process.env.DEVE_ANDROID_TARGET_MODEL,
});
const payload = JSON.stringify(facts);
const outputPath = process.env.DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH;
if (outputPath) {
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(facts, null, 2)}\n`, "utf8");
}
console.log(payload);

if (process.env.DEVE_MOBILE_ANDROID_EXPECT_WRITABLE !== "0" && !facts.supportBaseline) {
  console.error(
    `android-target-capability: writable evidence requires API 29+ and WebView 137+; observed ${payload}`,
  );
  process.exit(2);
}

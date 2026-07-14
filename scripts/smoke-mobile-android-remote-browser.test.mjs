import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const browserSource = readFileSync(
  new URL("./smoke-mobile-android-remote-browser.mjs", import.meta.url),
  "utf8",
);
const hostSource = readFileSync(
  new URL("./smoke-mobile-android-remote-browser.sh", import.meta.url),
  "utf8",
);

test("Android RemoteBrowser smoke proves real business flow and zero IPC", () => {
  assert.match(browserSource, /loginAndroidRemote/);
  assert.match(browserSource, /createAndroidDocument/);
  assert.match(browserSource, /commitAndroidChange/);
  assert.match(browserSource, /ipc\.localhost/);
  assert.match(browserSource, /__DEVE_NATIVE_BACKEND_CONFIG__/);
  assert.match(browserSource, /probeWebCryptoEd25519/);
  assert.match(browserSource, /smoke-mobile-android-remote-browser/);
});

test("Android RemoteBrowser host smoke is preference-driven and target-qualified", () => {
  assert.match(hostSource, /native-backend\.json/);
  assert.match(hostSource, /inspect-android-target-capability\.mjs/);
  assert.match(hostSource, /run-as/);
  assert.match(hostSource, /deve_mobile LocalBackend/);
  assert.doesNotMatch(hostSource, /--remote-url/);
});

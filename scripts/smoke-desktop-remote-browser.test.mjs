import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync(new URL("./smoke-desktop-remote-browser.mjs", import.meta.url), "utf8");
const hostSource = readFileSync(
  new URL("./check-desktop-remote-browser-smoke.ps1", import.meta.url),
  "utf8",
);
const installRootSource = readFileSync(
  new URL("./lib/desktop-install-root.ps1", import.meta.url),
  "utf8",
);

test("RemoteBrowser WebView proof covers facade, IPC, CSP, and real business UI", () => {
  assert.match(source, /__DEVE_NATIVE_BACKEND_CONFIG__/);
  assert.match(source, /ipc\\\.localhost|ipc\\.localhost/);
  assert.match(source, /content security policy/i);
  assert.match(source, /createAndEditDocument/);
  assert.match(source, /commitAndVerifyHistory/);
  assert.doesNotMatch(source, /__TAURI_INTERNALS__\?\.invoke\(/);
});

test("host smoke enters remote mode by preference and returns through the native window menu", () => {
  assert.match(hostSource, /native-backend\.json/);
  assert.match(hostSource, /Use Local Backend/);
  assert.match(hostSource, /GetMenuItemID/);
  assert.match(hostSource, /PostMessage/);
  assert.match(hostSource, /0x0111, # WM_COMMAND/);
  assert.match(hostSource, /RemoteBrowser started a local sidecar/);
  assert.match(hostSource, /local restart did not own exactly one sidecar/);
  assert.match(hostSource, /fresh endpoint\/session\/scope evidence/);
  assert.doesNotMatch(hostSource, /ArgumentList.*--remote-url/);
  assert.match(hostSource, /Environment\.Remove\("DEVE_NATIVE_REMOTE_URL"\)/);
  assert.match(hostSource, /UTF8Encoding\]\:\:new\(\$false\)/);
});

test("host smoke normalizes the work root before passing Node module paths", () => {
  assert.match(hostSource, /\$workRootPath = \[System\.IO\.Path\]::GetFullPath\(\$WorkRoot\)/);
  assert.match(hostSource, /Join-Path \$workRootPath "playwright-core"/);
  assert.doesNotMatch(hostSource, /Join-Path \$WorkRoot "playwright-core"/);
});

test("host smoke binds the Desktop binary to an explicit canonical install root", () => {
  assert.match(hostSource, /\[string\]\$InstallRoot/);
  assert.match(hostSource, /lib\/desktop-install-root\.ps1/);
  assert.match(hostSource, /Assert-DeveDesktopInstallRoot/);
  assert.match(installRootSource, /GetFinalPathNameByHandle/);
  assert.match(installRootSource, /Resolve-DeveCanonicalExistingPath/);
  assert.match(installRootSource, /Test-DeveCanonicalDescendant/);
  assert.match(installRootSource, /DesktopBinary must be a real child of InstallRoot/);
  assert.doesNotMatch(hostSource, /DeveNotebookInstallerSmoke/);
});

test("host smoke rejects wrong-root/path-escape markers and accepts only the fixed marker schema", () => {
  assert.match(installRootSource, /\.deve-desktop-install-root\.json/);
  assert.match(installRootSource, /deve-desktop-remote-browser-smoke/);
  assert.match(installRootSource, /install marker must be a real child of InstallRoot/);
  assert.match(installRootSource, /install marker schema is invalid/);
  assert.match(installRootSource, /install marker value is invalid/);
});

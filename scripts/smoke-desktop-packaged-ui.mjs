import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  commitAndVerifyHistory,
  createAndEditDocument,
  delay,
  findBundledLocalPage,
  waitForReady,
  waitUntil,
} from "./lib/desktop-webview-business-flow.mjs";

const timeoutMs = Number(process.env.DEVE_DESKTOP_PACKAGED_UI_TIMEOUT_MS ?? "60000");
const cdpEndpoint = process.env.DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT;
const authorityEvidencePath = process.env.DEVE_DESKTOP_LOCAL_AUTHORITY_EVIDENCE_PATH;
const playwrightRequire = createRequire(
  process.env.DEVE_DESKTOP_PACKAGED_UI_PLAYWRIGHT_REQUIRE_FROM ?? import.meta.url,
);

export function isDirectInvocation(argvPath = process.argv[1], moduleUrl = import.meta.url) {
  return Boolean(argvPath) && moduleUrl === pathToFileURL(resolve(argvPath)).href;
}

async function verifySettingsFocusTrap(page, assertNoErrors) {
  await page.locator("[data-deve-open-search-button=true]").click();
  const query = page.locator("[data-deve-search-input=true]");
  await query.waitFor({ state: "visible", timeout: timeoutMs });
  await query.fill(">settings");
  const settingsCommand = page
    .locator('[data-deve-search-result-action="run-command"]')
    .filter({ hasText: /Settings|设置/i })
    .first();
  await settingsCommand.waitFor({ state: "visible", timeout: timeoutMs });
  await settingsCommand.click();

  const modal = page.locator('[data-deve-settings-surface="modal"]');
  await modal.waitFor({ state: "visible", timeout: timeoutMs });
  assert.equal(await modal.getAttribute("role"), "dialog");
  assert.equal(await modal.getAttribute("aria-modal"), "true");
  await waitUntil("settings initial close-button focus", () => page.evaluate(
    () => document.activeElement?.getAttribute("data-deve-settings-close") === "icon",
  ));
  await delay(100);
  assertNoErrors("after Settings open");

  const focusables = modal.locator(
    'button:not([disabled]):visible, input:not([disabled]):visible, textarea:not([disabled]):visible, select:not([disabled]):visible, a[href]:visible, [tabindex]:not([tabindex="-1"]):visible',
  );
  const focusableCount = await focusables.count();
  assert.ok(focusableCount >= 2, "settings modal must expose at least two focusable controls");

  await focusables.last().focus();
  await page.keyboard.press("Tab");
  assert.equal(
    await focusables.first().evaluate((element) => document.activeElement === element),
    true,
    "Tab from the last control must wrap to the first",
  );
  assertNoErrors("after forward focus wrap");

  await page.keyboard.press("Shift+Tab");
  assert.equal(
    await focusables.last().evaluate((element) => document.activeElement === element),
    true,
    "Shift+Tab from the first control must wrap to the last",
  );
  assertNoErrors("after reverse focus wrap");

  await page.keyboard.press("Escape");
  await modal.waitFor({ state: "detached", timeout: timeoutMs });
  await delay(100);
  assertNoErrors("after Settings close");
}

async function main() {
  if (!cdpEndpoint) throw new Error("DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT is required");
  const { chromium } = playwrightRequire("playwright-core");
  const browser = await chromium.connectOverCDP(cdpEndpoint, { timeout: timeoutMs });
  const errors = [];
  const page = await findBundledLocalPage(browser);
  page.on("pageerror", (error) => errors.push(`pageerror: ${error.message}`));
  page.on("console", (message) => {
    if (message.type() === "error" && !message.text().includes("favicon.ico")) {
      errors.push(`console: ${message.text()}`);
    }
  });
  const assertNoErrors = (phase) => {
    assert.deepEqual(errors, [], `${phase} emitted errors: ${JSON.stringify(errors)}`);
  };

  console.log("desktop-packaged-ui-webview: waiting for native session");
  await waitForReady(page);
  const nativeBootstrap = await page.evaluate(() => globalThis.__DEVE_NATIVE_BOOTSTRAP ?? null);
  assert.equal(nativeBootstrap?.capabilities?.backend_preference_control, true);
  assert.equal(nativeBootstrap?.session_bound, true);
  const nativeEndpoint = new URL(nativeBootstrap?.http_base);
  assert.equal(nativeEndpoint.protocol, "http:");
  assert.ok(["127.0.0.1", "localhost"].includes(nativeEndpoint.hostname));
  assert.ok(Number(nativeEndpoint.port) > 0, "LocalBackend must bind a fresh loopback port");
  const scope = await page.locator("[data-deve-sync-status]").first().evaluate((element) => ({
    repoId: element.getAttribute("data-deve-repo-id"),
    scopeNonce: Number(element.getAttribute("data-deve-scope-nonce")),
  }));
  assert.ok(scope.repoId, "LocalBackend must expose the backend-projected repo scope");
  assert.ok(Number.isInteger(scope.scopeNonce) && scope.scopeNonce > 0);
  if (authorityEvidencePath) {
    writeFileSync(authorityEvidencePath, `${JSON.stringify({
      origin: new URL(page.url()).origin,
      httpBase: nativeBootstrap.http_base,
      sessionBound: nativeBootstrap.session_bound,
      ...scope,
    }, null, 2)}\n`, "utf8");
  }
  assert.equal(await page.locator("#login-username").count(), 0, "native session must bypass login UI");
  const stamp = Date.now();
  console.log("desktop-packaged-ui-webview: creating and editing document");
  await createAndEditDocument(
    page,
    `packaged-ui-${stamp}.md`,
    `Packaged native WebView smoke ${stamp}`,
  );
  console.log("desktop-packaged-ui-webview: committing and checking history");
  await commitAndVerifyHistory(page, `packaged ui smoke ${stamp}`);
  console.log("desktop-packaged-ui-webview: checking Settings focus trap");
  await verifySettingsFocusTrap(page, assertNoErrors);
  assertNoErrors("packaged WebView");
  console.log("desktop-packaged-ui-webview: ok");
}

if (isDirectInvocation()) {
  main().then(
    () => process.exit(0),
    (error) => {
      console.error(`desktop-packaged-ui-webview: ${error.stack ?? error.message}`);
      process.exit(1);
    },
  );
}

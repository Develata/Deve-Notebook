import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const timeoutMs = Number(process.env.DEVE_DESKTOP_PACKAGED_UI_TIMEOUT_MS ?? "60000");
const cdpEndpoint = process.env.DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT;
const playwrightRequire = createRequire(
  process.env.DEVE_DESKTOP_PACKAGED_UI_PLAYWRIGHT_REQUIRE_FROM ?? import.meta.url,
);

export function isDirectInvocation(argvPath = process.argv[1], moduleUrl = import.meta.url) {
  return Boolean(argvPath) && moduleUrl === pathToFileURL(resolve(argvPath)).href;
}

function delay(ms) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, ms));
}

async function waitUntil(label, predicate, timeout = timeoutMs) {
  const started = Date.now();
  let lastError;
  while (Date.now() - started < timeout) {
    try {
      const value = await predicate();
      if (value) return value;
    } catch (error) {
      lastError = error;
    }
    await delay(250);
  }
  throw new Error(`timeout waiting for ${label}${lastError ? `: ${lastError.message}` : ""}`);
}

async function findAppPage(browser) {
  return waitUntil("native WebView page", async () => {
    for (const context of browser.contexts()) {
      for (const page of context.pages()) {
        if (await page.locator("[data-deve-sync-status]").count()) return page;
      }
    }
    return null;
  });
}

async function waitForReady(page) {
  await page.waitForFunction(
    () => document.querySelector("[data-deve-sync-status]")?.getAttribute("data-deve-sync-status") === "ready",
    null,
    { timeout: timeoutMs },
  );
}

async function waitForWritableEditor(page) {
  const host = page.locator("[data-deve-editor-host=true]:visible").first();
  await host.waitFor({ state: "visible", timeout: timeoutMs });
  await waitUntil(
    "visible writable editor",
    async () => (await host.getAttribute("data-deve-editor-readonly")) === "false",
  );
}

async function createAndEditDocument(page, path, content) {
  await page.locator("[data-deve-new-doc-button=true]").click({ force: true });
  const input = page.locator("[data-deve-search-input=true]");
  await input.waitFor({ state: "visible", timeout: timeoutMs });
  await input.fill(`+${path}`);
  await page.locator('[data-deve-search-result-action="create-doc"]').first().click();
  await waitForWritableEditor(page);
  await waitUntil("editor bridge content API", () => page.evaluate(
    () => typeof window.getEditorContent === "function"
      && typeof window.getEditorContent() === "string",
  ));
  const cm = page.locator(".cm-content:visible").first();
  await cm.click({ force: true });
  await cm.pressSequentially(content, { delay: 2 });
  await page.waitForFunction(
    (expected) => typeof window.getEditorContent === "function"
      && window.getEditorContent()?.includes(expected),
    content,
    { timeout: timeoutMs },
  );
  await page.waitForFunction(
    () => document.querySelector("[data-deve-pending-ack-count]")
      ?.getAttribute("data-deve-pending-ack-count") === "0",
    null,
    { timeout: timeoutMs },
  );
}

async function openSourceControl(page) {
  await page.locator("[data-deve-activity-more-button]").click();
  await page.locator('[data-deve-activity-more-item="activity_more_item_source_control"]').click();
}

async function commitAndVerifyHistory(page, message) {
  await openSourceControl(page);
  const textarea = page.locator('textarea[name="commit-message"]');
  await textarea.waitFor({ state: "visible", timeout: timeoutMs });
  await waitUntil("commit message input enabled", () => textarea.isEnabled());
  await textarea.fill(message);
  const commitPanel = textarea.locator("xpath=ancestor::div[contains(@class,'border-t')][1]");
  const commitButton = commitPanel.locator("button:has(.codicon-check)").first();
  await waitUntil("commit action enabled", () => commitButton.isEnabled());
  await commitButton.click();
  await waitUntil("commit message cleared", async () => (await textarea.inputValue()) === "");

  const historyToggle = page.locator('[data-deve-sc-panel-toggle="history"]');
  if ((await historyToggle.getAttribute("aria-expanded")) !== "true") {
    await historyToggle.click();
  }
  const history = page.locator('[data-deve-sc-panel-body="history"]');
  await history.waitFor({ state: "visible", timeout: timeoutMs });
  await waitUntil("commit visible in history", async () => (await history.innerText()).includes(message));
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
  const page = await findAppPage(browser);
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

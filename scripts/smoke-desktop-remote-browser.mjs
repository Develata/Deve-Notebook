import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { writeFileSync } from "node:fs";
import {
  commitAndVerifyHistory,
  createAndEditDocument,
  waitForReady,
  waitUntil,
} from "./lib/desktop-webview-business-flow.mjs";

const timeoutMs = Number(process.env.DEVE_DESKTOP_PACKAGED_UI_TIMEOUT_MS ?? "60000");
const cdpEndpoint = process.env.DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT;
const remoteOrigin = process.env.DEVE_DESKTOP_REMOTE_HTTPS_ORIGIN;
const username = process.env.DEVE_DESKTOP_REMOTE_USERNAME;
const password = process.env.DEVE_DESKTOP_REMOTE_PASSWORD;
const authorityEvidencePath = process.env.DEVE_DESKTOP_REMOTE_AUTHORITY_EVIDENCE_PATH;
const playwrightRequire = createRequire(
  process.env.DEVE_DESKTOP_PACKAGED_UI_PLAYWRIGHT_REQUIRE_FROM ?? import.meta.url,
);

async function findRemotePage(browser) {
  return waitUntil("RemoteBrowser WebView page", async () => {
    for (const context of browser.contexts()) {
      for (const page of context.pages()) {
        if (page.url().startsWith(remoteOrigin)) return page;
      }
    }
    return null;
  });
}

async function main() {
  if (!cdpEndpoint || !remoteOrigin || !username || !password) {
    throw new Error("CDP endpoint, remote HTTPS origin, username, and password are required");
  }
  assert.equal(new URL(remoteOrigin).protocol, "https:");
  const { chromium } = playwrightRequire("playwright-core");
  const browser = await chromium.connectOverCDP(cdpEndpoint, { timeout: timeoutMs });
  const page = await findRemotePage(browser);
  const networkUrls = [];
  const consoleErrors = [];
  page.on("request", (request) => networkUrls.push(request.url()));
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });

  // Attach diagnostics before a fresh navigation so initial bridge/CSP behavior
  // is observed instead of inferred from a page that loaded before CDP attached.
  await page.reload({ waitUntil: "domcontentloaded", timeout: timeoutMs });
  await page.locator("#login-username, [data-deve-sync-status]").first()
    .waitFor({ state: "attached", timeout: timeoutMs });
  assert.equal(new URL(page.url()).origin, new URL(remoteOrigin).origin);

  const bridge = await page.evaluate(() => ({
    capability: globalThis.__DEVE_NATIVE_BOOTSTRAP?.capabilities
      ?.backend_preference_control === true,
    facade: Boolean(globalThis.__deveWebBridge?.get?.("__DEVE_NATIVE_BACKEND_CONFIG__")),
    directFacade: Boolean(globalThis.__DEVE_NATIVE_BACKEND_CONFIG__),
  }));
  assert.equal(bridge.capability, false, "RemoteBrowser must not receive local capability");
  assert.equal(bridge.facade, false, "RemoteBrowser must not register native backend facade");
  assert.equal(bridge.directFacade, false, "RemoteBrowser must not expose a direct native facade");

  const usernameInput = page.locator("#login-username");
  if (await usernameInput.isVisible({ timeout: 15000 }).catch(() => false)) {
    await usernameInput.fill(username);
    await page.locator("#login-password").fill(password);
    await page.locator('button[type="submit"]').click();
  }
  await waitForReady(page);

  const stamp = Date.now();
  await createAndEditDocument(
    page,
    `desktop-remote-${stamp}.md`,
    `Desktop RemoteBrowser smoke ${stamp}`,
  );
  await commitAndVerifyHistory(page, `desktop remote smoke ${stamp}`);
  const scope = await page.locator("[data-deve-sync-status]").first().evaluate((element) => ({
    repoId: element.getAttribute("data-deve-repo-id"),
    scopeNonce: Number(element.getAttribute("data-deve-scope-nonce")),
  }));
  assert.ok(scope.repoId, "RemoteBrowser must expose the backend-projected repo scope");
  assert.ok(Number.isInteger(scope.scopeNonce) && scope.scopeNonce > 0);
  if (authorityEvidencePath) {
    writeFileSync(authorityEvidencePath, `${JSON.stringify({
      origin: new URL(page.url()).origin,
      ...scope,
    }, null, 2)}\n`, "utf8");
  }

  const ipcRequests = networkUrls.filter((url) => {
    try {
      return new URL(url).hostname === "ipc.localhost";
    } catch {
      return url.includes("ipc.localhost");
    }
  });
  const ipcCspErrors = consoleErrors.filter((message) =>
    /ipc\.localhost|content security policy|refused to connect/i.test(message));
  assert.deepEqual(ipcRequests, [], `RemoteBrowser emitted native IPC requests: ${ipcRequests}`);
  assert.deepEqual(ipcCspErrors, [], `RemoteBrowser emitted IPC/CSP errors: ${ipcCspErrors}`);
  console.log("desktop-remote-browser-webview: ok");
}

main().then(
  () => process.exit(0),
  (error) => {
    console.error(`desktop-remote-browser-webview: ${error.stack ?? error.message}`);
    process.exit(1);
  },
);

import assert from "node:assert/strict";
import { createRequire } from "node:module";

const playwrightRequire = createRequire(
  process.env.DEVE_DOCKER_MULTI_PLAYWRIGHT_REQUIRE_FROM ?? import.meta.url,
);
const { chromium } = playwrightRequire("playwright");

const baseUrl = process.env.DEVE_DOCKER_MULTI_BASE_URL ?? "http://127.0.0.1:3101";
const authUser = process.env.DEVE_DOCKER_MULTI_AUTH_USER ?? "admin";
const authPassword = process.env.DEVE_DOCKER_MULTI_AUTH_PASSWORD ?? "password";
const headless = !["0", "false", "no"].includes(
  (process.env.DEVE_DOCKER_MULTI_HEADLESS ?? "1").toLowerCase(),
);
const timeoutMs = Number(process.env.DEVE_DOCKER_MULTI_TIMEOUT_MS ?? "60000");

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitUntil(label, predicate, timeout = timeoutMs) {
  const started = Date.now();
  let lastError;
  while (Date.now() - started < timeout) {
    try {
      if (await predicate()) {
        return;
      }
    } catch (err) {
      lastError = err;
    }
    await delay(250);
  }
  const suffix = lastError ? `: ${lastError.message}` : "";
  throw new Error(`timeout waiting for ${label}${suffix}`);
}

function attachDiagnostics(page, label) {
  const diag = {
    label,
    wsUrls: [],
    responses: [],
    consoleErrors: [],
    pageErrors: [],
  };

  page.on("websocket", (ws) => {
    diag.wsUrls.push(ws.url());
  });
  page.on("response", (response) => {
    const url = new URL(response.url());
    if (url.pathname.startsWith("/api/")) {
      diag.responses.push({ path: url.pathname, status: response.status() });
    }
  });
  page.on("console", (msg) => {
    if (msg.type() === "error") {
      diag.consoleErrors.push(msg.text());
    }
  });
  page.on("pageerror", (err) => {
    diag.pageErrors.push(err.message);
  });

  return diag;
}

function relevantConsoleErrors(diag) {
  return diag.consoleErrors.filter((message) => {
    if (message.includes("favicon.ico")) {
      return false;
    }
    if (message.includes("net::ERR_INTERNET_DISCONNECTED")) {
      return false;
    }
    return true;
  });
}

function assertApiResponse(diag, path, expectedStatus) {
  const matched = diag.responses.some(
    (response) => response.path === path && response.status === expectedStatus,
  );
  assert.ok(
    matched,
    `${diag.label} did not observe ${path} -> ${expectedStatus}; observed ${JSON.stringify(
      diag.responses,
    )}`,
  );
}

function hasRelativeWs(diag) {
  return diag.wsUrls.some((url) => {
    try {
      return new URL(url).pathname === "/ws";
    } catch {
      return false;
    }
  });
}

async function assertPageHealthy(page, diag) {
  const body = await page.locator("body").innerText({ timeout: 10000 });
  assert.ok(body.trim().length > 0, `${diag.label} rendered a blank page`);
  assert.doesNotMatch(
    body,
    /Vite|Internal Server Error|Module not found|failed to load module script/i,
    `${diag.label} rendered a framework error overlay`,
  );
  const consoleErrors = relevantConsoleErrors(diag);
  assert.equal(
    consoleErrors.length,
    0,
    `${diag.label} has console errors: ${JSON.stringify(consoleErrors)}`,
  );
  assert.equal(
    diag.pageErrors.length,
    0,
    `${diag.label} has uncaught page errors: ${JSON.stringify(diag.pageErrors)}`,
  );
}

async function waitForStatus(page, allowed, timeout = timeoutMs) {
  await page.waitForFunction(
    (allowedStatuses) => {
      const el = document.querySelector("[data-deve-sync-status]");
      const status = el?.getAttribute("data-deve-sync-status");
      return status != null && allowedStatuses.includes(status);
    },
    allowed,
    { timeout },
  );
  return page.locator("[data-deve-sync-status]").first().getAttribute("data-deve-sync-status");
}

async function waitForReady(page, label) {
  try {
    await waitForStatus(page, ["ready"], timeoutMs);
  } catch (err) {
    const retry = page.locator("[data-deve-peer-registration-retry=true]");
    if (await retry.isVisible().catch(() => false)) {
      await retry.click();
      await waitForStatus(page, ["ready"], timeoutMs);
      return;
    }
    throw new Error(`${label} did not become ready: ${err.message}`);
  }
}

async function login(page, diag) {
  await page.goto(baseUrl, { waitUntil: "domcontentloaded" });
  await assertPageHealthy(page, diag);

  const username = page.locator("#login-username");
  if (await username.isVisible({ timeout: 15000 }).catch(() => false)) {
    await username.fill(authUser);
    await page.locator("#login-password").fill(authPassword);
    await page.locator('button[type="submit"]').click();
  }

  await waitForReady(page, diag.label);
  await waitUntil(`${diag.label} relative /ws`, () => hasRelativeWs(diag), 30000);
  assertApiResponse(diag, "/api/node/role", 200);
  assertApiResponse(diag, "/api/auth/status", 200);
  await assertPageHealthy(page, diag);
}

async function openSearch(page) {
  await page.locator("[data-deve-open-search-button=true]").click();
  const input = page.locator("[data-deve-search-input=true]");
  await input.waitFor({ state: "visible", timeout: 10000 });
  return input;
}

async function waitForWritableEditor(page) {
  await page.locator("[data-deve-editor-host=true]").waitFor({ state: "visible", timeout: timeoutMs });
  await page.waitForFunction(
    () => document
      .querySelector("[data-deve-editor-host=true]")
      ?.getAttribute("data-deve-editor-readonly") === "false",
    null,
    { timeout: timeoutMs },
  );
}

async function editorContent(page) {
  return page.evaluate(() => {
    if (typeof window.getEditorContent !== "function") {
      return null;
    }
    return window.getEditorContent();
  });
}

async function waitForEditorContains(page, expected) {
  await page.waitForFunction(
    (value) => typeof window.getEditorContent === "function"
      && window.getEditorContent().includes(value),
    expected,
    { timeout: timeoutMs },
  );
}

async function createDocAndType(page, path, content) {
  await waitForReady(page, "client-a before create");
  await page.locator("[data-deve-new-doc-button=true]").click({ force: true });
  const input = page.locator("[data-deve-search-input=true]");
  await input.waitFor({ state: "visible", timeout: 10000 });
  await input.fill(`+${path}`);
  await page.locator('[data-deve-search-result-action="create-doc"]').first().click();

  await waitForWritableEditor(page);
  const cm = page.locator(".cm-content").first();
  await cm.click();
  await page.keyboard.type(content);
  await waitForEditorContains(page, content);
  await waitForStatus(page, ["ready"], timeoutMs);
}

async function openDoc(page, path) {
  const input = await openSearch(page);
  await input.fill(path);
  const result = page
    .locator('[data-deve-search-result-action="open-doc"]')
    .filter({ hasText: path })
    .first();
  await result.waitFor({ state: "visible", timeout: timeoutMs });
  await result.click();
  await waitForWritableEditor(page);
}

async function exerciseOfflineRecovery(context, page, diag) {
  const wsCountBefore = diag.wsUrls.length;
  await context.setOffline(true);
  await page.locator('[data-deve-disconnect-overlay="lockdown"]').waitFor({
    state: "visible",
    timeout: 20000,
  });
  await waitForStatus(page, ["offline", "reconnecting"], 20000);
  await page.waitForFunction(
    () => document
      .querySelector("[data-deve-editor-host=true]")
      ?.getAttribute("data-deve-editor-readonly") === "true",
    null,
    { timeout: 20000 },
  );

  await context.setOffline(false);
  await waitForStatus(page, ["ready"], timeoutMs);
  await page.locator('[data-deve-disconnect-overlay="lockdown"]').waitFor({
    state: "detached",
    timeout: 20000,
  }).catch(() => {});
  await waitUntil(
    `${diag.label} reconnect websocket`,
    () => diag.wsUrls.length > wsCountBefore && hasRelativeWs(diag),
    timeoutMs,
  );
}

async function main() {
  const docPath = `docker-multiclient-${Date.now()}.md`;
  const content = `Docker multiclient smoke ${new Date().toISOString()}`;
  const browser = await chromium.launch({ headless });
  const contexts = [];

  try {
    const contextA = await browser.newContext();
    const contextB = await browser.newContext();
    contexts.push(contextA, contextB);
    const pageA = await contextA.newPage();
    const pageB = await contextB.newPage();
    const diagA = attachDiagnostics(pageA, "client-a");
    const diagB = attachDiagnostics(pageB, "client-b");

    await login(pageA, diagA);
    await login(pageB, diagB);

    await createDocAndType(pageA, docPath, content);
    await openDoc(pageB, docPath);
    await waitForEditorContains(pageB, content);
    assert.equal(await editorContent(pageB), content, "client-b content must match client-a edit");

    await exerciseOfflineRecovery(contextB, pageB, diagB);
    await waitForEditorContains(pageB, content);
    await assertPageHealthy(pageA, diagA);
    await assertPageHealthy(pageB, diagB);

    console.log(JSON.stringify({
      status: "ok",
      baseUrl,
      docPath,
      clients: [
        { label: diagA.label, ws: diagA.wsUrls.length },
        { label: diagB.label, ws: diagB.wsUrls.length },
      ],
    }));
  } finally {
    for (const context of contexts.reverse()) {
      await context.close().catch(() => {});
    }
    await browser.close().catch(() => {});
  }
}

main().catch((err) => {
  console.error("docker-multiclient-smoke: Playwright failure");
  console.error(err);
  process.exit(1);
});

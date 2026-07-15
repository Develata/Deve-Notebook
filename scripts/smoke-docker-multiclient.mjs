import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  exerciseRepoLifecycle,
  exerciseSourceControlAndExternalChanges,
} from "./lib/docker-multiclient-product-journeys.mjs";

const playwrightRequire = createRequire(
  process.env.DEVE_DOCKER_MULTI_PLAYWRIGHT_REQUIRE_FROM ?? import.meta.url,
);
const baseUrl = process.env.DEVE_DOCKER_MULTI_BASE_URL ?? "http://127.0.0.1:3101";
const expectedOrigin = process.env.DEVE_DOCKER_MULTI_EXPECTED_ORIGIN ?? baseUrl;
const authUser = process.env.DEVE_DOCKER_MULTI_AUTH_USER ?? "admin";
const authPassword = process.env.DEVE_DOCKER_MULTI_AUTH_PASSWORD ?? "password";
const headless = !["0", "false", "no"].includes(
  (process.env.DEVE_DOCKER_MULTI_HEADLESS ?? "1").toLowerCase(),
);
const timeoutMs = Number(process.env.DEVE_DOCKER_MULTI_TIMEOUT_MS ?? "60000");
const productJourneys = ["1", "true"].includes(
  (process.env.DEVE_DOCKER_MULTI_PRODUCT_JOURNEYS ?? "0").toLowerCase(),
);
export const renderedShellSelector = "#login-username, [data-deve-sync-status]";

export function renderedShellPresent(selector, root = document) {
  return root.querySelector(selector) != null;
}

export async function waitForRenderedShell(page, timeout = 15000) {
  await page.waitForFunction(renderedShellPresent, renderedShellSelector, { timeout });
}

export function isDirectInvocation(argvPath = process.argv[1], moduleUrl = import.meta.url) {
  return Boolean(argvPath) && moduleUrl === pathToFileURL(resolve(argvPath)).href;
}

export function editorContentIncludes(expected, root = window) {
  if (typeof root.getEditorContent !== "function") {
    return false;
  }
  const content = root.getEditorContent();
  return typeof content === "string" && content.includes(expected);
}

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
    offline: false,
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
      const message = msg.text();
      diag.consoleErrors.push({ message, duringOffline: diag.offline });
      if (!diag.offline) {
        console.error(`docker-multiclient-smoke: ${label} console error: ${message}`);
      }
    }
  });
  page.on("pageerror", (err) => {
    const detail = err.stack || err.message;
    diag.pageErrors.push(detail);
    console.error(`docker-multiclient-smoke: ${label} page error: ${detail}`);
  });

  return diag;
}

export function relevantConsoleErrors(diag) {
  return diag.consoleErrors.filter(({ message, duringOffline }) => {
    if (message.includes("favicon.ico")) {
      return false;
    }
    if (duringOffline && message.includes("net::ERR_INTERNET_DISCONNECTED")) {
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

export function webSocketMatchesExpectedOrigin(url, httpOrigin = expectedOrigin) {
  try {
    const expected = new URL(httpOrigin);
    expected.protocol = expected.protocol === "https:" ? "wss:" : "ws:";
    const observed = new URL(url);
    return observed.origin === expected.origin && observed.pathname === "/ws";
  } catch {
    return false;
  }
}

function hasRelativeWs(diag) {
  return diag.wsUrls.some((url) => {
    try {
      return webSocketMatchesExpectedOrigin(url);
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
  assert.equal(
    new URL(page.url()).origin,
    new URL(expectedOrigin).origin,
    `${diag.label} loaded an unexpected origin`,
  );
  try {
    await waitForRenderedShell(page, timeoutMs);
  } catch (err) {
    await assertPageHealthy(page, diag);
    throw new Error(`${diag.label} did not render the login or sync shell: ${err.message}`);
  }
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

export async function pendingAckCount(page) {
  return page.locator("[data-deve-pending-ack-count]").first().evaluate((element) => {
    const raw = element.getAttribute("data-deve-pending-ack-count");
    const value = Number(raw);
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new Error(`invalid pending ack count marker: ${raw}`);
    }
    return value;
  });
}

async function waitForEditorContains(page, expected) {
  await page.waitForFunction(
    editorContentIncludes,
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

async function exerciseOfflineRecovery(context, page, diag, peerPage) {
  const contentBefore = await editorContent(page);
  const peerContentBefore = await editorContent(peerPage);
  const pendingBefore = await pendingAckCount(page);
  const wsCountBefore = diag.wsUrls.length;
  diag.offline = true;
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

  const blockedInput = ` offline-blocked-${Date.now()}`;
  const cm = page.locator(".cm-content").first();
  await cm.click({ force: true });
  await page.keyboard.press("Control+End");
  await page.keyboard.type(blockedInput);
  await delay(300);
  assert.equal(await editorContent(page), contentBefore, "offline input must not change local editor content");
  assert.equal(await pendingAckCount(page), pendingBefore, "offline input must not enqueue pending edits");
  assert.equal(await editorContent(peerPage), peerContentBefore, "offline input must not reach the peer editor");

  await context.setOffline(false);
  diag.offline = false;
  await waitForStatus(page, ["ready"], timeoutMs);
  await page.locator('[data-deve-disconnect-overlay="lockdown"]').waitFor({
    state: "detached",
    timeout: 20000,
  });
  await waitUntil(
    `${diag.label} reconnect websocket`,
    () => diag.wsUrls.slice(wsCountBefore).some((url) => webSocketMatchesExpectedOrigin(url)),
    timeoutMs,
  );
  await waitForWritableEditor(page);
}

async function appendEditorContent(page, content) {
  const cm = page.locator(".cm-content").first();
  await cm.click();
  await page.keyboard.press("Control+End");
  await page.keyboard.type(content);
  await waitForEditorContains(page, content);
  await waitForStatus(page, ["ready"], timeoutMs);
}

async function main() {
  const docPath = `docker-multiclient-${Date.now()}.md`;
  const content = `Docker multiclient smoke ${new Date().toISOString()}`;
  const recoveryContent = ` reconnect-write-${Date.now()}`;
  const { chromium } = playwrightRequire("playwright");
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

    let repoLifecycle = null;
    if (productJourneys) {
      repoLifecycle = await exerciseRepoLifecycle(pageA);
    }

    await createDocAndType(pageA, docPath, content);
    await openDoc(pageB, docPath);
    await waitForEditorContains(pageB, content);
    assert.equal(await editorContent(pageB), content, "client-b content must match client-a edit");

    await exerciseOfflineRecovery(contextB, pageB, diagB, pageA);
    await waitForEditorContains(pageB, content);
    await appendEditorContent(pageB, recoveryContent);
    await waitForEditorContains(pageA, recoveryContent);
    assert.ok(
      (await editorContent(pageA))?.includes(recoveryContent),
      "client-a must receive client-b's post-reconnect edit",
    );
    let productEvidence = null;
    if (productJourneys) {
      productEvidence = await exerciseSourceControlAndExternalChanges({
        page: pageA,
        peerPage: pageB,
        repoId: repoLifecycle.initialRepoId,
        path: docPath,
        currentContent: `${content}${recoveryContent}`,
      });
    }
    await assertPageHealthy(pageA, diagA);
    await assertPageHealthy(pageB, diagB);

    console.log(JSON.stringify({
      status: "ok",
      baseUrl,
      docPath,
      repoLifecycle,
      productEvidence,
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

if (isDirectInvocation()) {
  main().catch((err) => {
    console.error("docker-multiclient-smoke: Playwright failure");
    console.error(err);
    process.exit(1);
  });
}

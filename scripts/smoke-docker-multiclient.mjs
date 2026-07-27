import assert from "node:assert/strict";
import { createRequire } from "node:module";
import {
  assertNoScope,
  assertRemovalPreservation,
  createFirstRepoFromNoScope,
  exerciseLastRepoNoScope,
  exerciseRepoLifecycle,
  exerciseSourceControlAndExternalChanges,
  restartCandidateContainer,
} from "./lib/docker-multiclient-product-journeys.mjs";
import {
  attachDiagnostics,
  beginHostRestart,
  closeBrowserResources,
  delay,
  editorContentIncludes,
  endHostRestart,
  isDirectInvocation,
  readNodeRole,
  relevantConsoleErrors,
  relevantRequestFailures,
  renderedShellPresent,
  renderedShellSelector,
  waitForRestartedNodeRole,
  waitForRenderedShell,
  waitUntil as waitUntilWithTimeout,
  webSocketMatchesExpectedOrigin as runtimeWebSocketMatchesExpectedOrigin,
} from "./lib/docker-multiclient-runtime.mjs";

export {
  editorContentIncludes,
  isDirectInvocation,
  renderedShellPresent,
  renderedShellSelector,
  relevantConsoleErrors,
  waitForRenderedShell,
};

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
async function waitUntil(label, predicate, timeout = timeoutMs) {
  return waitUntilWithTimeout(label, predicate, timeout);
}

export function webSocketMatchesExpectedOrigin(url, httpOrigin = expectedOrigin) {
  return runtimeWebSocketMatchesExpectedOrigin(url, httpOrigin);
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
  return diag.sockets.some(({ url }) => webSocketMatchesExpectedOrigin(url, expectedOrigin));
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
  const requestFailures = relevantRequestFailures(diag);
  assert.equal(
    requestFailures.length,
    0,
    `${diag.label} has unexpected request failures: ${JSON.stringify(requestFailures)}`,
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

async function reopenNoScope(page, diag) {
  await page.goto("about:blank");
  await delay(50);
  const socketCountBefore = diag.sockets.length;
  await page.goto(baseUrl, { waitUntil: "domcontentloaded" });
  await waitForRenderedShell(page, timeoutMs);
  const username = page.locator("#login-username");
  if (await username.isVisible({ timeout: 5000 }).catch(() => false)) {
    await username.fill(authUser);
    await page.locator("#login-password").fill(authPassword);
    await page.locator('button[type="submit"]').click();
  }
  await waitUntil(
    `${diag.label} current-navigation websocket server frame`,
    () => diag.sockets
      .slice(socketCountBefore)
      .some(({ url, frames }) => webSocketMatchesExpectedOrigin(url) && frames > 0),
  );
  await assertNoScope(page, `${diag.label} after restart`);
}

async function reopenReady(page, diag) {
  const socketCountBefore = diag.sockets.length;
  await page.goto(baseUrl, { waitUntil: "domcontentloaded" });
  await waitForRenderedShell(page, timeoutMs);
  const username = page.locator("#login-username");
  if (await username.isVisible({ timeout: 5000 }).catch(() => false)) {
    await username.fill(authUser);
    await page.locator("#login-password").fill(authPassword);
    await page.locator('button[type="submit"]').click();
  }
  await waitForReady(page, `${diag.label} after phase restart`);
  await waitUntil(
    `${diag.label} phase-restart websocket server frame`,
    () => diag.sockets
      .slice(socketCountBefore)
      .some(({ url, frames }) => webSocketMatchesExpectedOrigin(url) && frames > 0),
  );
  await assertPageHealthy(page, diag);
}

async function restartBetweenProductPhases(pages) {
  const roleBeforeRestart = await readNodeRole(baseUrl);
  await Promise.all(pages.map((page) => page.goto("about:blank")));
  restartCandidateContainer();
  const roleAfterRestart = await waitForRestartedNodeRole({
    baseUrl,
    before: roleBeforeRestart,
    timeoutMs,
  });
  assert.notEqual(
    roleAfterRestart.runtime_incarnation,
    roleBeforeRestart.runtime_incarnation,
    "product phase restart must replace the process runtime",
  );
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
  const socketCountBefore = diag.sockets.length;
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
    () => diag.sockets
      .slice(socketCountBefore)
      .some(({ url, frames }) => webSocketMatchesExpectedOrigin(url) && frames > 0),
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
    const diagA = attachDiagnostics(pageA, "client-a", expectedOrigin);
    const diagB = attachDiagnostics(pageB, "client-b", expectedOrigin);

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
    let lastRepoEvidence = null;
    let recreatedRepo = null;
    if (productJourneys) {
      productEvidence = await exerciseSourceControlAndExternalChanges({
        page: pageA,
        peerPage: pageB,
        repoId: repoLifecycle.initialRepoId,
        path: docPath,
        currentContent: `${content}${recoveryContent}`,
      });

      // The host-side Playwright contexts share one Docker gateway IP. Separate
      // the high-traffic collaboration/source-control phase from the removal
      // phase without weakening the product's 120 requests/minute/IP contract.
      // Volumes stay mounted, so this also proves the first phase is durable.
      await restartBetweenProductPhases([pageA, pageB]);
      await reopenReady(pageA, diagA);
      await reopenReady(pageB, diagB);

      const mobileContext = await browser.newContext({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        isMobile: true,
      });
      contexts.push(mobileContext);
      const mobilePage = await mobileContext.newPage();
      const mobileDiag = attachDiagnostics(
        mobilePage,
        "client-mobile-390x844",
        expectedOrigin,
      );
      await login(mobilePage, mobileDiag);
      lastRepoEvidence = await exerciseLastRepoNoScope({
        page: mobilePage,
        observerPages: [pageA, pageB],
      });

      const roleBeforeRestart = await readNodeRole(baseUrl);
      beginHostRestart([diagA, diagB, mobileDiag]);
      restartCandidateContainer();
      const roleAfterRestart = await waitForRestartedNodeRole({
        baseUrl,
        before: roleBeforeRestart,
        timeoutMs,
      });
      await endHostRestart([diagA, diagB, mobileDiag]);
      await reopenNoScope(pageA, diagA);
      await reopenNoScope(pageB, diagB);
      await reopenNoScope(mobilePage, mobileDiag);
      assertRemovalPreservation(
        lastRepoEvidence.removedRepoId,
        lastRepoEvidence.preservation,
      );
      recreatedRepo = await createFirstRepoFromNoScope(mobilePage, [pageA, pageB]);
      assert.notEqual(recreatedRepo.repoId, lastRepoEvidence.removedRepoId);
      assertRemovalPreservation(
        lastRepoEvidence.removedRepoId,
        lastRepoEvidence.preservation,
      );
      await assertPageHealthy(mobilePage, mobileDiag);
      productEvidence = {
        ...productEvidence,
        mobileViewport: mobilePage.viewportSize(),
        mobileServerFrames: mobileDiag.sockets
          .reduce((total, socket) => total + socket.frames, 0),
        restartRuntimeChanged:
          roleAfterRestart.runtime_incarnation !== roleBeforeRestart.runtime_incarnation,
      };
    }
    await assertPageHealthy(pageA, diagA);
    await assertPageHealthy(pageB, diagB);

    console.log(JSON.stringify({
      status: "ok",
      baseUrl,
      docPath,
      repoLifecycle,
      productEvidence,
      lastRepoEvidence,
      recreatedRepo,
      clients: [
        { label: diagA.label, ws: diagA.sockets.length },
        { label: diagB.label, ws: diagB.sockets.length },
      ],
    }));
  } finally {
    await closeBrowserResources(contexts, browser);
  }
}

if (isDirectInvocation(process.argv[1], import.meta.url)) {
  main().catch((err) => {
    console.error("docker-multiclient-smoke: Playwright failure");
    console.error(err);
    process.exit(1);
  });
}

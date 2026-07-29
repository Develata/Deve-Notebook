import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  assertRemovalPreservation,
  configureFirstRepoProjectionBase,
  prepareRemovalPreservationFixture,
} from "./docker-multiclient-workspace.mjs";

const timeoutMs = Number(process.env.DEVE_DOCKER_MULTI_TIMEOUT_MS ?? "60000");

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitUntil(label, predicate, timeout = timeoutMs) {
  const started = Date.now();
  let lastError;
  while (Date.now() - started < timeout) {
    try {
      if (await predicate()) return;
    } catch (error) {
      lastError = error;
    }
    await delay(250);
  }
  throw new Error(`timeout waiting for ${label}${lastError ? `: ${lastError.message}` : ""}`);
}

async function scope(page) {
  return page.locator("[data-deve-sync-status]").first().evaluate((element) => ({
    repoId: element.getAttribute("data-deve-repo-id"),
    status: element.getAttribute("data-deve-sync-status"),
  }));
}

async function ensureRepoSwitcherVisible(page) {
  const trigger = page.locator("[data-deve-repo-switcher-trigger]").first();
  if (await trigger.isVisible().catch(() => false)) {
    return trigger;
  }
  const openDrawer = page.locator('[data-deve-mobile-header-action="open_left_drawer"]');
  await openDrawer.click();
  await trigger.waitFor({ state: "visible", timeout: timeoutMs });
  return trigger;
}

async function openRepoSwitcher(page) {
  const trigger = await ensureRepoSwitcherVisible(page);
  await trigger.click();
  await page.locator('[data-deve-repo-switcher-menu="visible"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
}

function pageViewport(page) {
  return page.viewportSize();
}

async function assertRemovalDialogContract(page, dialog, expectedAlias) {
  const text = await dialog.innerText();
  assert.ok(
    text.includes(expectedAlias),
    "removal preview must identify the backend-selected display alias",
  );
  await dialog.locator("#repo-removal-deleted-heading").waitFor({ state: "visible" });
  await dialog.locator("#repo-removal-preserved-heading").waitFor({ state: "visible" });
  assert.ok(
    await dialog.locator("#repo-removal-deleted-heading + ul > li").count() > 0,
    "removal preview must render backend-owned deleted categories",
  );
  assert.ok(
    await dialog.locator("#repo-removal-preserved-heading + ul > li").count() > 0,
    "removal preview must render backend-owned preserved categories",
  );
  const confirm = dialog.locator('[data-deve-repo-removal-confirm="true"]');
  assert.equal(await confirm.isEnabled(), true, "unblocked removal must be backend-enabled");
  const box = await dialog.locator('[role="dialog"]').boundingBox();
  const viewport = pageViewport(page);
  if (box && viewport) {
    assert.ok(box.x >= 0 && box.x + box.width <= viewport.width + 1);
    assert.ok(box.y >= 0 && box.y + box.height <= viewport.height + 1);
  }
}

export async function exerciseRepoLifecycle(page) {
  const initial = await scope(page);
  assert.ok(initial.repoId, "initial repo scope must expose a repo id");
  const repoName = `browser-smoke-${Date.now()}`;
  await openRepoSwitcher(page);
  await page.locator("[data-deve-repo-switcher-create]").click();
  const createInput = page.locator("[data-deve-repo-switcher-create-input]");
  await createInput.fill(repoName);
  await createInput.press("Enter");
  await waitUntil("new repository scope", async () => {
    const current = await scope(page);
    return current.status === "ready" && current.repoId && current.repoId !== initial.repoId;
  });
  const created = await scope(page);
  assert.ok(created.repoId, "created repo scope must expose a repo id");
  const createdPreservation = prepareRemovalPreservationFixture(created.repoId);

  await openRepoSwitcher(page);
  await page
    .locator('[data-deve-repo-switcher-item-name="default"]')
    .click();
  await waitUntil("switch back to default repository", async () => {
    const current = await scope(page);
    return current.status === "ready" && current.repoId === initial.repoId;
  });

  await openRepoSwitcher(page);
  const createdRow = page
    .locator(`[data-deve-repo-switcher-item-name="${repoName}"]`)
    .locator("xpath=..");
  await createdRow.locator("[data-deve-repo-switcher-actions]").click();
  await page.locator("[data-deve-repo-switcher-remove]").click();
  const dialog = page.locator('[data-deve-repo-removal-dialog="visible"]');
  await dialog.waitFor({ state: "visible", timeout: timeoutMs });
  await assertRemovalDialogContract(page, dialog, repoName);
  await dialog.locator('[data-deve-repo-removal-confirm="true"]').click();
  await dialog.waitFor({ state: "hidden", timeout: timeoutMs });
  await waitUntil("removed repository absent from switcher", async () =>
    (await page.locator(`[data-deve-repo-switcher-item-name="${repoName}"]`).count()) === 0);
  await waitUntil("fallback repository ready after removal", async () => {
    const current = await scope(page);
    return current.repoId === initial.repoId && current.status === "ready";
  });
  const afterRemoval = await scope(page);
  assert.equal(afterRemoval.repoId, initial.repoId);
  assert.equal(afterRemoval.status, "ready");
  assertRemovalPreservation(created.repoId, createdPreservation);
  return { initialRepoId: initial.repoId, removedRepoId: created.repoId };
}

export async function assertNoScope(page, label) {
  await waitUntil(`${label} NoScope`, async () => {
    const current = await scope(page);
    return current.repoId === "";
  });
  await openRepoSwitcher(page);
  assert.equal(
    await page.locator("[data-deve-repo-switcher-item]").count(),
    0,
    `${label} NoScope must expose no local repo rows`,
  );
  await page.locator("[data-deve-repo-switcher-create]").waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  await page.locator("[data-deve-repo-switcher-backdrop]").click();
}

export async function exerciseLastRepoNoScope({ page, observerPages }) {
  const viewport = pageViewport(page);
  assert.deepEqual(viewport, { width: 390, height: 844 });
  const before = await scope(page);
  assert.ok(before.repoId, "mobile last-repo journey requires one selected repo");
  configureFirstRepoProjectionBase();
  const preservation = prepareRemovalPreservationFixture(before.repoId);

  await openRepoSwitcher(page);
  const rows = page.locator("[data-deve-repo-switcher-item]");
  assert.equal(await rows.count(), 1, "last-repo journey requires exactly one repo");
  const row = rows.first().locator("xpath=..");
  const alias = (await rows.first().getAttribute("data-deve-repo-switcher-item-name")) ?? "";
  assert.ok(alias, "last repo must expose a display alias");
  await row.locator("[data-deve-repo-switcher-actions]").click();
  await page.locator("[data-deve-repo-switcher-remove]").click();
  const dialog = page.locator('[data-deve-repo-removal-dialog="visible"]');
  await dialog.waitFor({ state: "visible", timeout: timeoutMs });
  await assertRemovalDialogContract(page, dialog, alias);
  await dialog.locator("#repo-removal-warnings-heading").waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  await dialog.locator('[data-deve-repo-removal-confirm="true"]').click();
  await dialog.waitFor({ state: "hidden", timeout: timeoutMs });

  await assertNoScope(page, "mobile client");
  for (const [index, observer] of observerPages.entries()) {
    await assertNoScope(observer, `desktop observer ${index + 1}`);
  }
  assertRemovalPreservation(before.repoId, preservation);
  const overflow = await page.evaluate(() => ({
    width: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
  assert.equal(overflow.width, 390);
  assert.ok(
    overflow.scrollWidth <= overflow.width,
    `mobile NoScope overflowed horizontally: ${JSON.stringify(overflow)}`,
  );
  return { removedRepoId: before.repoId, preservation };
}

export function restartCandidateContainer() {
  const docker = process.env.DEVE_DOCKER_MULTI_DOCKER_BIN ?? "docker";
  const container = process.env.DEVE_DOCKER_MULTI_CONTAINER_ID;
  assert.ok(container, "DEVE_DOCKER_MULTI_CONTAINER_ID is required for product journeys");
  execFileSync(docker, ["restart", container], {
    stdio: ["ignore", "ignore", "pipe"],
    timeout: 60000,
  });
}

export async function createFirstRepoFromNoScope(page, observerPages = []) {
  const name = `after-restart-${Date.now()}`;
  await openRepoSwitcher(page);
  await page.locator("[data-deve-repo-switcher-create]").click();
  const input = page.locator("[data-deve-repo-switcher-create-input]");
  await input.fill(name);
  await input.press("Enter");
  await waitUntil("first repo created after NoScope restart", async () => {
    const current = await scope(page);
    return current.status === "ready" && Boolean(current.repoId);
  });
  const created = await scope(page);
  assert.ok(created.repoId);
  await openRepoSwitcher(page);
  const creatorRow = page.locator(`[data-deve-repo-switcher-item-name="${name}"]`);
  await waitUntil("created repo alias in creator list", async () =>
    (await creatorRow.count()) === 1);
  await page.locator("[data-deve-repo-switcher-backdrop]").click();

  for (const [index, observer] of observerPages.entries()) {
    await openRepoSwitcher(observer);
    const row = observer.locator(`[data-deve-repo-switcher-item-name="${name}"]`);
    await waitUntil(`created repo alias in observer ${index + 1}`, async () =>
      (await row.count()) === 1);
    await row.click();
    await waitUntil(`created repo scope in observer ${index + 1}`, async () => {
      const current = await scope(observer);
      return current.status === "ready" && current.repoId === created.repoId;
    });
  }
  return { name, repoId: created.repoId };
}

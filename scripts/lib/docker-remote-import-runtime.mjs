import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import {
  attachDiagnostics,
  relevantConsoleErrors,
  relevantRequestFailures,
  waitForRenderedShell,
  waitUntil,
} from "./docker-multiclient-runtime.mjs";

const requireFrom = process.env.DEVE_REMOTE_IMPORT_PLAYWRIGHT_REQUIRE_FROM ?? import.meta.url;
export const { chromium } = createRequire(requireFrom)("playwright");
export const timeoutMs = Number(process.env.DEVE_REMOTE_IMPORT_TIMEOUT_MS ?? "90000");

export function runDocker(args, options = {}) {
  const executable = process.env.DEVE_REMOTE_IMPORT_DOCKER_BIN ?? "docker";
  const result = spawnSync(executable, args, {
    encoding: "utf8",
    shell: false,
    env: process.env,
    timeout: options.timeout ?? timeoutMs,
  });
  if (result.error || result.status !== 0) {
    throw new Error(
      `docker ${args.join(" ")} failed: ${result.error?.message ?? result.stderr ?? result.stdout}`,
    );
  }
  return result.stdout.trim();
}

export function compose(args) {
  return runDocker([
    "compose",
    "-f",
    process.env.DEVE_REMOTE_IMPORT_COMPOSE_FILE,
    "-p",
    process.env.DEVE_REMOTE_IMPORT_PROJECT,
    ...args,
  ]);
}

export function execCandidate(container, script) {
  return runDocker(["exec", container, "sh", "-eu", "-c", script]);
}

export async function login(page, baseUrl, label) {
  const diag = attachDiagnostics(page, label, baseUrl);
  await page.goto(baseUrl, { waitUntil: "domcontentloaded" });
  await waitForRenderedShell(page, timeoutMs);
  const username = page.locator("#login-username");
  if (await username.isVisible({ timeout: 10000 }).catch(() => false)) {
    await username.fill(process.env.DEVE_REMOTE_IMPORT_AUTH_USER);
    await page.locator("#login-password").fill(process.env.DEVE_REMOTE_IMPORT_AUTH_PASSWORD);
    await page.locator('button[type="submit"]').click();
  }
  await page.locator('[data-deve-sync-status="ready"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  return diag;
}

export async function reopen(page, baseUrl) {
  await page.goto("about:blank");
  await page.goto(baseUrl, { waitUntil: "domcontentloaded" });
  await waitForRenderedShell(page, timeoutMs);
  const username = page.locator("#login-username");
  if (await username.isVisible({ timeout: 5000 }).catch(() => false)) {
    await username.fill(process.env.DEVE_REMOTE_IMPORT_AUTH_USER);
    await page.locator("#login-password").fill(process.env.DEVE_REMOTE_IMPORT_AUTH_PASSWORD);
    await page.locator('button[type="submit"]').click();
  }
  await page.locator('[data-deve-sync-status="ready"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
}

export async function openRemoteImport(page) {
  await page.locator("[data-deve-activity-more-button]").click();
  await page
    .locator('[data-deve-activity-more-item="activity_more_item_remote_import"]')
    .click();
  await page.locator('[data-deve-remote-import-view="true"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
}

export async function refreshRemoteImport(page) {
  await page.locator("[data-deve-activity-more-button]").click();
  await page
    .locator('[data-deve-activity-more-item="activity_more_item_explorer"]')
    .click();
  await page.locator('[data-deve-remote-import-view="true"]').waitFor({
    state: "detached",
    timeout: timeoutMs,
  });
  await openRemoteImport(page);
}

export async function openRepoSwitcher(page) {
  let trigger = page.locator("[data-deve-repo-switcher-trigger]").first();
  if (!(await trigger.isVisible().catch(() => false))) {
    await page.locator("[data-deve-activity-more-button]").click();
    await page
      .locator('[data-deve-activity-more-item="activity_more_item_explorer"]')
      .click();
    trigger = page.locator("[data-deve-repo-switcher-trigger]").first();
    await trigger.waitFor({ state: "visible", timeout: timeoutMs });
  }
  await trigger.click();
  await page.locator('[data-deve-repo-switcher-menu="visible"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
}

export async function assertNoSensitiveProjection(page) {
  const text = await page.locator("body").innerText();
  for (const value of [
    process.env.DEVE_REMOTE_IMPORT_S3_ACCESS_KEY_ID,
    process.env.DEVE_REMOTE_IMPORT_S3_SECRET_ACCESS_KEY,
    process.env.DEVE_REMOTE_IMPORT_WEBDAV_ORIGIN,
    process.env.DEVE_REMOTE_IMPORT_S3_ORIGIN,
  ]) {
    if (value) assert.ok(!text.includes(value), "browser projection exposed provider material");
  }
}

export function assertDiagnostics(diag) {
  assert.deepEqual(relevantConsoleErrors(diag), [], `${diag.label} console errors`);
  assert.deepEqual(relevantRequestFailures(diag), [], `${diag.label} request failures`);
  assert.deepEqual(diag.pageErrors, [], `${diag.label} page errors`);
}

export async function currentRepo(page) {
  const status = page.locator("[data-deve-sync-status]").first();
  return {
    repoId: await status.getAttribute("data-deve-repo-id"),
    state: await status.getAttribute("data-deve-sync-status"),
  };
}

export async function createRepo(page, name) {
  await openRepoSwitcher(page);
  await page.locator("[data-deve-repo-switcher-create]").click();
  const input = page.locator("[data-deve-repo-switcher-create-input]");
  await input.fill(name);
  await input.press("Enter");
  await waitUntil(
    "created repo scope",
    async () => {
      const scope = await currentRepo(page);
      return scope.state === "ready" && Boolean(scope.repoId);
    },
    timeoutMs,
  );
}

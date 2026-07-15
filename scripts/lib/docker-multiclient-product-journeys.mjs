import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

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

export async function exerciseRepoLifecycle(page) {
  const initial = await scope(page);
  assert.ok(initial.repoId, "initial repo scope must expose a repo id");
  const repoName = `browser-smoke-${Date.now()}`;
  await page.locator("[data-deve-repo-switcher-trigger]").click();
  await page.locator("[data-deve-repo-switcher-create]").click();
  const createInput = page.locator("[data-deve-repo-switcher-create-input]");
  await createInput.fill(repoName);
  await createInput.press("Enter");
  await waitUntil("new repository scope", async () => {
    const current = await scope(page);
    return current.status === "ready" && current.repoId && current.repoId !== initial.repoId;
  });

  await page.locator("[data-deve-repo-switcher-trigger]").click();
  await page
    .locator('[data-deve-repo-switcher-item-name="default"]')
    .click();
  await waitUntil("switch back to default repository", async () => {
    const current = await scope(page);
    return current.status === "ready" && current.repoId === initial.repoId;
  });
  return { initialRepoId: initial.repoId, createdRepo: repoName };
}

async function openActivity(page, item) {
  await page.locator("[data-deve-activity-more-button]").click();
  await page.locator(`[data-deve-activity-more-item="${item}"]`).click();
}

async function assertNoBrowserErrorOverlay(page, context) {
  const overlay = page.locator("#loading-overlay");
  if (await overlay.isVisible()) {
    const detail = (await overlay.innerText()).trim();
    throw new Error(`${context}: browser error overlay is visible: ${detail}`);
  }
}

async function openTypedDiff(page, row) {
  await row.click();
  await page.locator('[data-deve-diff-projection="backend-typed"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  await page.locator('[data-deve-diff-viewport="virtualized"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  await page.locator('[data-deve-mobile-diff-action="diff-close-button"]').click();
}

async function commitAndVerifyHistory(page, path, message) {
  await openActivity(page, "activity_more_item_source_control");
  const confirmed = page
    .locator('[data-deve-sc-section-body="confirmed-ledger"] [data-deve-mobile-touch-target="source-control-change-row"]')
    .filter({ hasText: path })
    .first();
  await confirmed.waitFor({ state: "visible", timeout: timeoutMs });
  await openTypedDiff(page, confirmed);

  const textarea = page.locator('textarea[name="commit-message"]');
  await textarea.waitFor({ state: "visible", timeout: timeoutMs });
  await waitUntil("commit message input enabled", () => textarea.isEnabled());
  await textarea.fill(message);
  const commitButton = page.locator('[data-deve-source-control-commit-action="commit"]');
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

export function selectWorkspaceRoot(output, repoId) {
  const roots = output
    .split(/\r?\n/u)
    .map((value) => value.trim())
    .filter(Boolean);
  const matches = roots.filter((root) => root.endsWith(`--${repoId}`));
  assert.equal(matches.length, 1, `expected one workspace for repo ${repoId}, observed ${JSON.stringify(roots)}`);
  assert.match(matches[0], /^\/notes\/[a-zA-Z0-9._-]+--[0-9a-f-]+$/u);
  return matches[0];
}

function mutateWorkspaceFile(repoId, path, content) {
  const docker = process.env.DEVE_DOCKER_MULTI_DOCKER_BIN ?? "docker";
  const container = process.env.DEVE_DOCKER_MULTI_CONTAINER_ID;
  assert.ok(container, "DEVE_DOCKER_MULTI_CONTAINER_ID is required for product journeys");
  const workspaceOutput = execFileSync(
    docker,
    ["exec", container, "find", "/notes", "-mindepth", "1", "-maxdepth", "1", "-type", "d"],
    { encoding: "utf8", timeout: 30000 },
  );
  const workspace = selectWorkspaceRoot(workspaceOutput, repoId);
  execFileSync(
    docker,
    ["exec", "-i", container, "tee", `${workspace}/${path}`],
    { input: content, stdio: ["pipe", "ignore", "pipe"], timeout: 30000 },
  );
}

async function editorContent(page) {
  return page.evaluate(() => typeof window.getEditorContent === "function"
    ? window.getEditorContent()
    : null);
}

async function applyExternalChange(page, peerPage, repoId, path, externalContent) {
  const before = await editorContent(page);
  const peerBefore = await editorContent(peerPage);
  mutateWorkspaceFile(repoId, path, externalContent);
  await delay(500);
  assert.equal(await editorContent(page), before, "external workspace mutation must not bypass ledger authority");
  assert.equal(await editorContent(peerPage), peerBefore, "external workspace mutation must not reach another client before apply");

  await openActivity(page, "activity_more_item_external_changes");
  const pending = page
    .locator('[data-deve-external-section-body="pending"] [data-deve-external-changes-row]')
    .filter({ hasText: path })
    .first();
  await pending.waitFor({ state: "visible", timeout: timeoutMs });
  assert.equal(await editorContent(page), before, "detected external change must remain outside ledger authority");
  assert.equal(await editorContent(peerPage), peerBefore, "detected external change must remain invisible to peers before apply");
  await openTypedDiff(page, pending);
  await assertNoBrowserErrorOverlay(page, "after closing External Changes diff");
  const stage = pending.locator('[data-deve-external-action="stage"]');
  await stage.click();

  const staged = page
    .locator('[data-deve-external-section-body="staged"] [data-deve-external-changes-row]')
    .filter({ hasText: path })
    .first();
  await staged.waitFor({ state: "visible", timeout: timeoutMs });
  const apply = page.locator('[data-deve-external-apply="true"]');
  await waitUntil("Apply to Ledger enabled", () => apply.isEnabled());
  await apply.click();
  await waitUntil("external content applied to first client", async () =>
    (await editorContent(page)) === externalContent);
  await waitUntil("external content synchronized to peer", async () =>
    (await editorContent(peerPage)) === externalContent);

  await openActivity(page, "activity_more_item_source_control");
  const confirmed = page
    .locator('[data-deve-sc-section-body="confirmed-ledger"] [data-deve-mobile-touch-target="source-control-change-row"]')
    .filter({ hasText: path })
    .first();
  await confirmed.waitFor({ state: "visible", timeout: timeoutMs });
}

export async function exerciseSourceControlAndExternalChanges({
  page,
  peerPage,
  repoId,
  path,
  currentContent,
}) {
  const commitMessage = `docker browser commit ${Date.now()}`;
  await commitAndVerifyHistory(page, path, commitMessage);
  const externalContent = `${currentContent}\nexternal projection ${Date.now()}\n`;
  await applyExternalChange(page, peerPage, repoId, path, externalContent);
  return { commitMessage, externalContent };
}

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
  const created = await scope(page);
  assert.ok(created.repoId, "created repo scope must expose a repo id");
  const createdWorkspace = prepareRemovalPreservationFixture(created.repoId);

  await page.locator("[data-deve-repo-switcher-trigger]").click();
  await page
    .locator('[data-deve-repo-switcher-item-name="default"]')
    .click();
  await waitUntil("switch back to default repository", async () => {
    const current = await scope(page);
    return current.status === "ready" && current.repoId === initial.repoId;
  });

  await page.locator("[data-deve-repo-switcher-trigger]").click();
  const createdRow = page
    .locator(`[data-deve-repo-switcher-item-name="${repoName}"]`)
    .locator("xpath=..");
  await createdRow.locator("[data-deve-repo-switcher-actions]").click();
  await createdRow.locator("[data-deve-repo-switcher-remove]").click();
  const dialog = page.locator('[data-deve-repo-removal-dialog="visible"]');
  await dialog.waitFor({ state: "visible", timeout: timeoutMs });
  assert.ok(
    (await dialog.innerText()).includes(repoName),
    "removal preview must identify the backend-selected display alias",
  );
  await dialog.locator('[data-deve-repo-removal-confirm="true"]').click();
  await dialog.waitFor({ state: "hidden", timeout: timeoutMs });
  await waitUntil("removed repository absent from switcher", async () =>
    (await page.locator(`[data-deve-repo-switcher-item-name="${repoName}"]`).count()) === 0);
  const afterRemoval = await scope(page);
  assert.equal(afterRemoval.repoId, initial.repoId);
  assert.equal(afterRemoval.status, "ready");
  assertRemovalPreservation(created.repoId, createdWorkspace);
  return { initialRepoId: initial.repoId, removedRepoId: created.repoId };
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

function locatorStringField(block, field) {
  const match = block.match(new RegExp(`^${field}\\s*=\\s*(['"])([^'"\\r\\n]+)\\1\\s*$`, "mu"));
  assert.ok(match, `Projection Locator record is missing ${field}`);
  return match[2];
}

export function selectWorkspaceRoot(locatorContent, repoId) {
  assert.match(repoId, /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u);
  const records = locatorContent.split(/^\[\[locators\]\]\s*$/mu).slice(1).map((block) => ({
    repoId: locatorStringField(block, "repo_id"),
    segment: locatorStringField(block, "workspace_segment"),
    base: locatorStringField(block, "projection_base_abs"),
  }));
  const matches = records.filter((record) => record.repoId === repoId);
  assert.equal(matches.length, 1, `expected one locator for repo ${repoId}`);
  const [{ base, segment }] = matches;
  assert.equal(base, "/notes", `Docker smoke projection base must be /notes, observed ${base}`);
  assert.match(segment, /^(?:[a-zA-Z0-9._-]+--)?[0-9a-f-]+$/u);
  return `${base}/${segment}`;
}

export function validateWorkspaceIdentity(identityContent, repoId) {
  assert.match(identityContent, /^version\s*=\s*1\s*$/mu);
  assert.equal(locatorStringField(identityContent, "repo_id"), repoId);
}

function dockerWorkspaceRoot(repoId) {
  const docker = process.env.DEVE_DOCKER_MULTI_DOCKER_BIN ?? "docker";
  const container = process.env.DEVE_DOCKER_MULTI_CONTAINER_ID;
  assert.ok(container, "DEVE_DOCKER_MULTI_CONTAINER_ID is required for product journeys");
  const locatorContent = execFileSync(
    docker,
    ["exec", container, "cat", "/data/ledger/.host/projection-locators.toml"],
    { encoding: "utf8", timeout: 30000 },
  );
  return {
    docker,
    container,
    workspace: selectWorkspaceRoot(locatorContent, repoId),
  };
}

function prepareRemovalPreservationFixture(repoId) {
  const { docker, container, workspace } = dockerWorkspaceRoot(repoId);
  execFileSync(
    docker,
    [
      "exec",
      container,
      "sh",
      "-c",
      'set -eu; root="$1"; printf "# preserved\\n" > "$root/preserved.md"; printf "unknown\\n" > "$root/unknown.bin"; mkdir -p "$root/.git"; printf "[core]\\n" > "$root/.git/config"',
      "_",
      workspace,
    ],
    { stdio: ["ignore", "ignore", "pipe"], timeout: 30000 },
  );
  return workspace;
}

function assertRemovalPreservation(repoId, workspace) {
  const docker = process.env.DEVE_DOCKER_MULTI_DOCKER_BIN ?? "docker";
  const container = process.env.DEVE_DOCKER_MULTI_CONTAINER_ID;
  assert.ok(container, "DEVE_DOCKER_MULTI_CONTAINER_ID is required for product journeys");
  execFileSync(
    docker,
    [
      "exec",
      container,
      "sh",
      "-c",
      'set -eu; root="$1"; repo_id="$2"; test -f "$root/preserved.md"; test -f "$root/unknown.bin"; test -f "$root/.git/config"; test -f "$root/.gitignore"; test ! -e "$root/.notegit"; test ! -e "/data/ledger/local/${repo_id}.redb"',
      "_",
      workspace,
      repoId,
    ],
    { stdio: ["ignore", "ignore", "pipe"], timeout: 30000 },
  );
}

function mutateWorkspaceFile(repoId, path, content) {
  const { docker, container, workspace } = dockerWorkspaceRoot(repoId);
  const identityContent = execFileSync(
    docker,
    ["exec", container, "sh", "-c", 'test ! -L "$1" && test -f "$1" && cat "$1"', "_", `${workspace}/.notegit/identity.toml`],
    { encoding: "utf8", timeout: 30000 },
  );
  validateWorkspaceIdentity(identityContent, repoId);
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

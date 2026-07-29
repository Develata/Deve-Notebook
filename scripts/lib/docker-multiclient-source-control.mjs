import assert from "node:assert/strict";
import { mutateWorkspaceFile } from "./docker-multiclient-workspace.mjs";

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

async function editorContent(page) {
  return page.evaluate(() => typeof window.getEditorContent === "function"
    ? window.getEditorContent()
    : null);
}

async function applyExternalChange(page, peerPage, repoId, path, externalContent, expectedContent) {
  // The preceding Source Control commit broadcast makes both clients re-open
  // the doc; sampling the authority baseline mid-rehydration reads an empty
  // editor and fakes a ledger bypass. Anchor the baseline to the known
  // document content instead of trusting a racy snapshot.
  await waitUntil("first client editor settled before external mutation", async () =>
    (await editorContent(page)) === expectedContent);
  await waitUntil("peer editor settled before external mutation", async () =>
    (await editorContent(peerPage)) === expectedContent);
  const before = expectedContent;
  const peerBefore = expectedContent;
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
  const stageResponsePromise = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return url.pathname === "/api/sc/stage-pending"
      && response.request().method() === "POST";
  }, { timeout: timeoutMs });
  await stage.click();
  const stageResponse = await stageResponsePromise;
  assert.equal(
    stageResponse.status(),
    204,
    `External Changes stage returned ${stageResponse.status()}`,
  );

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
  await applyExternalChange(page, peerPage, repoId, path, externalContent, currentContent);
  return { commitMessage, externalContent };
}

import assert from "node:assert/strict";
import { existsSync, writeFileSync } from "node:fs";
import {
  assertNoSensitiveProjection,
  createRepo,
  execCandidate,
  openRemoteImport,
  openRepoSwitcher,
  refreshRemoteImport,
  reopen,
  runDocker,
  timeoutMs,
} from "./docker-remote-import-runtime.mjs";
import {
  beginHostRestart,
  endHostRestart,
  waitUntil,
} from "./docker-multiclient-runtime.mjs";

async function waitForHttp(url) {
  await waitUntil(
    url,
    async () => {
      try {
        return (await fetch(url, { signal: AbortSignal.timeout(2000) })).ok;
      } catch {
        return false;
      }
    },
    timeoutMs,
  );
}

async function selectSession(page, sessionId) {
  const row = page.locator(`[data-deve-remote-import-session="${sessionId}"]`);
  await row.waitFor({ state: "visible", timeout: timeoutMs });
  await row.click();
  await page.locator(`[data-deve-remote-import-selected="${sessionId}"]`).waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  await page.locator('[data-deve-remote-import-entries="backend-typed"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
}

async function discardOnlyFailedSession(page, baseUrl) {
  await reopen(page, baseUrl);
  await openRemoteImport(page);
  const rows = page.locator("[data-deve-remote-import-session]");
  await waitUntil(
    "one failed Remote Import session",
    async () => (await rows.count()) === 1,
    timeoutMs,
  );
  const row = rows.first();
  const sessionId = await row.getAttribute("data-deve-remote-import-session");
  assert.ok(sessionId, "failed Remote Import session must expose its backend identity");
  const failedProjection = await row.innerText();
  await row.click();
  await page.locator(`[data-deve-remote-import-selected="${sessionId}"]`).waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  await page.locator('[data-deve-remote-import-discard="true"]').click();
  await waitUntil(
    "failed Remote Import session discarded",
    async () => (await row.innerText()) !== failedProjection,
    timeoutMs,
  );
  return sessionId;
}

async function prepareWebDavWithBoundedRetry(page) {
  const observedSessionIds = new Set(
    await page
      .locator("[data-deve-remote-import-session]")
      .evaluateAll((nodes) =>
        nodes
          .map((node) => node.getAttribute("data-deve-remote-import-session"))
          .filter(Boolean),
      ),
  );
  for (let attempt = 1; attempt <= 5; attempt += 1) {
    const prepare = page.locator('[data-deve-remote-import-prepare="webdav"]');
    const selected = page.locator("[data-deve-remote-import-selected]");
    const typedError = page.locator('[data-deve-remote-import-error="typed"]');
    await waitUntil(
      `WebDAV Prepare attempt ${attempt} admitted`,
      async () => !(await prepare.isDisabled()),
      timeoutMs,
    );
    const beforeSelectedId = await selected
      .getAttribute("data-deve-remote-import-selected")
      .catch(() => null);
    await prepare.click();
    await waitUntil(
      `WebDAV Prepare attempt ${attempt} settled`,
      async () => {
        const selectedId = await selected
          .getAttribute("data-deve-remote-import-selected")
          .catch(() => null);
        return (
          (selectedId !== null && selectedId !== beforeSelectedId) ||
          ((await typedError.count()) === 1 && (await typedError.isVisible()))
        );
      },
      timeoutMs,
    );
    const selectedId = await selected
      .getAttribute("data-deve-remote-import-selected")
      .catch(() => null);
    if (selectedId !== null && selectedId !== beforeSelectedId) {
      return selected;
    }
    assert.equal(
      selectedId,
      beforeSelectedId,
      `WebDAV Prepare attempt ${attempt} failure must not install a Ready selection`,
    );
    await refreshRemoteImport(page);
    const reopenedRows = page.locator("[data-deve-remote-import-session]");
    await waitUntil(
      `WebDAV Prepare attempt ${attempt} failure persisted`,
      async () => {
        const sessionIds = await reopenedRows.evaluateAll((nodes) =>
          nodes
            .map((node) => node.getAttribute("data-deve-remote-import-session"))
            .filter(Boolean),
        );
        return sessionIds.some((sessionId) => !observedSessionIds.has(sessionId));
      },
      timeoutMs,
    );
    const newSessionIds = (
      await reopenedRows.evaluateAll((nodes) =>
        nodes
          .map((node) => node.getAttribute("data-deve-remote-import-session"))
          .filter(Boolean),
      )
    ).filter((sessionId) => !observedSessionIds.has(sessionId));
    assert.equal(
      newSessionIds.length,
      1,
      `WebDAV Prepare attempt ${attempt} must persist exactly one new failed session`,
    );
    const failedSessionId = newSessionIds[0];
    observedSessionIds.add(failedSessionId);
    const failedRow = page.locator(
      `[data-deve-remote-import-session="${failedSessionId}"]`,
    );
    const failedProjection = await failedRow.innerText();
    await failedRow.click();
    await page
      .locator(`[data-deve-remote-import-selected="${failedSessionId}"]`)
      .waitFor({ state: "visible", timeout: timeoutMs });
    await page.locator('[data-deve-remote-import-discard="true"]').click();
    await waitUntil(
      `WebDAV Prepare attempt ${attempt} failure discarded`,
      async () => (await failedRow.innerText()) !== failedProjection,
      timeoutMs,
    );
    await refreshRemoteImport(page);
  }
  throw new Error("healthy WebDAV Prepare exhausted the bounded retry budget");
}

async function reopenSession(page, baseUrl, sessionId) {
  await reopen(page, baseUrl);
  await openRemoteImport(page);
  await selectSession(page, sessionId);
}

async function openEntryDiff(page, label) {
  const row = page
    .locator("[data-deve-remote-import-entry]")
    .filter({ hasText: label })
    .first();
  await row.waitFor({ state: "visible", timeout: timeoutMs });
  await row.click();
  const diff = page.locator('[data-deve-remote-import-diff="backend-typed"]');
  await diff.waitFor({ state: "visible", timeout: timeoutMs });
  await diff.locator('[data-deve-diff-projection="backend-typed"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  return diff;
}

async function restartCandidate(page, diag, baseUrl, container) {
  beginHostRestart([diag]);
  runDocker(["restart", container], { timeout: timeoutMs });
  await waitForHttp(`${baseUrl}/api/node/role`);
  endHostRestart([diag]);
  await reopen(page, baseUrl);
}

export async function exerciseWebDavFailure(page) {
  const baseUrl = process.env.DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_BASE_URL;
  await openRemoteImport(page);
  await page.locator('[data-deve-remote-import-prepare="webdav"]').click();
  await page.locator('[data-deve-remote-import-error="typed"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  assert.equal(
    await page.locator("[data-deve-remote-import-selected]").count(),
    0,
    "provider failure must not install a Ready selection",
  );
  await discardOnlyFailedSession(page, baseUrl);
  assert.equal(
    await page.locator('[data-deve-remote-import-apply="true"]').isDisabled(),
    true,
    "failed and discarded provider session must not become applyable",
  );
  await assertNoSensitiveProjection(page);
}

export async function exerciseWebDav(page, diag) {
  const baseUrl = process.env.DEVE_REMOTE_IMPORT_WEBDAV_BASE_URL;
  const container = process.env.DEVE_REMOTE_IMPORT_WEBDAV_APP_CONTAINER;
  const mutationFile = process.env.DEVE_REMOTE_IMPORT_WEBDAV_MUTATION_FILE;
  assert.ok(mutationFile, "WebDAV scenario requires an owned fixture target");
  assert.equal(
    existsSync(mutationFile),
    true,
    "healthy WebDAV provider target must exist before Prepare",
  );
  await openRemoteImport(page);

  const selected = await prepareWebDavWithBoundedRetry(page);
  const sessionId = await selected.getAttribute("data-deve-remote-import-selected");
  assert.ok(sessionId, "WebDAV Prepare must install a backend session identity");
  execCandidate(
    container,
    "test -z \"$(find /notes -type f -name '*.md' -print -quit)\"",
  );

  await restartCandidate(page, diag, baseUrl, container);
  await openRemoteImport(page);
  await selectSession(page, sessionId);
  const beforeRefresh = await openEntryDiff(page, "webdav-sealed.md");
  const sealedText = await beforeRefresh.innerText();
  assert.match(sealedText, /sealed-before-refresh/u);

  writeFileSync(
    mutationFile,
    "# WebDAV provider receipt\nprovider-mutated-after-prepare\n",
    { encoding: "utf8" },
  );
  await page.locator('[data-deve-mobile-diff-action="diff-close-button"]').click();
  const sessionRow = page.locator(
    `[data-deve-remote-import-session="${sessionId}"]`,
  );
  const beforeRefreshProjection = await sessionRow.innerText();
  await page.locator('[data-deve-remote-import-refresh="true"]').click();
  await waitUntil(
    "WebDAV refresh projection changed",
    async () => (await sessionRow.innerText()) !== beforeRefreshProjection,
    timeoutMs,
  );
  await reopenSession(page, baseUrl, sessionId);
  const afterRefresh = await openEntryDiff(page, "webdav-sealed.md");
  const refreshedText = await afterRefresh.innerText();
  assert.match(refreshedText, /sealed-before-refresh/u);
  assert.doesNotMatch(refreshedText, /provider-mutated-after-prepare/u);

  await page.locator('[data-deve-mobile-diff-action="diff-close-button"]').click();
  const beforeDiscardProjection = await sessionRow.innerText();
  await page.locator('[data-deve-remote-import-discard="true"]').click();
  await waitUntil(
    "WebDAV session discarded",
    async () => (await sessionRow.innerText()) !== beforeDiscardProjection,
    timeoutMs,
  );
  assert.equal(
    await page.locator('[data-deve-remote-import-apply="true"]').isDisabled(),
    true,
    "Discarded session must not remain applyable",
  );
  await assertNoSensitiveProjection(page);
}

export async function exerciseS3(page, diag) {
  const baseUrl = process.env.DEVE_REMOTE_IMPORT_S3_BASE_URL;
  const container = process.env.DEVE_REMOTE_IMPORT_S3_APP_CONTAINER;
  const initialRepoId = await page
    .locator("[data-deve-sync-status]")
    .first()
    .getAttribute("data-deve-repo-id");
  assert.ok(initialRepoId, "S3 scenario requires an initial repo scope");
  await openRemoteImport(page);
  await page.locator('[data-deve-remote-import-prepare="s3"]').click();
  const selected = page.locator("[data-deve-remote-import-selected]");
  await selected.waitFor({ state: "visible", timeout: timeoutMs });
  const sessionId = await selected.getAttribute("data-deve-remote-import-selected");
  assert.ok(sessionId, "S3 Prepare must install a backend session identity");
  execCandidate(
    container,
    "test -z \"$(find /notes -type f -name '*.md' -print -quit)\"",
  );

  await reopenSession(page, baseUrl, sessionId);
  const diff = await openEntryDiff(page, "s3-applied.md");
  assert.match(await diff.innerText(), /sealed-before-apply/u);
  await page.locator('[data-deve-mobile-diff-action="diff-close-button"]').click();
  await page.locator('[data-deve-remote-import-apply="true"]').click();
  await page.locator('[data-deve-remote-import-apply-outcome="backend-typed"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  execCandidate(
    container,
    "test \"$(find /notes -type f -name '*.md' | wc -l)\" -eq 2 && test \"$(find /notes -type f -name s3-applied.md | wc -l)\" -eq 1 && test \"$(find /notes -type f -path '*/nested/shared.md' | wc -l)\" -eq 1 && grep -R -q sealed-before-apply /notes && grep -R -q backend-owned-diff /notes",
  );

  await restartCandidate(page, diag, baseUrl, container);
  await openRemoteImport(page);
  await selectSession(page, sessionId);
  await page.locator('[data-deve-remote-import-apply-outcome="backend-typed"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });

  await createRepo(page, `b6-no-locator-${Date.now()}`);
  const secondRepoId = await page
    .locator("[data-deve-sync-status]")
    .first()
    .getAttribute("data-deve-repo-id");
  assert.notEqual(secondRepoId, initialRepoId, "repo create must switch exact scope");
  await openRemoteImport(page);
  assert.equal(
    await page.locator("[data-deve-remote-import-session]").count(),
    0,
    "new repo scope must retire all prior session summaries immediately",
  );
  await page.locator('[data-deve-remote-import-prepare="s3"]').click();
  await page.locator('[data-deve-remote-import-error="typed"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  assert.equal(
    await page.locator("[data-deve-remote-import-selected]").count(),
    0,
    "repo without locator must not inherit another repo session",
  );
  assert.equal(
    await page.locator("[data-deve-remote-import-session]").count(),
    0,
    "repo without locator must keep the session list isolated after failure",
  );

  await openRepoSwitcher(page);
  await page.locator('[data-deve-repo-switcher-item-name="default"]').click();
  await waitUntil(
    "return to original S3 repo",
    async () =>
      (await page
        .locator("[data-deve-sync-status]")
        .first()
        .getAttribute("data-deve-repo-id")) === initialRepoId,
    timeoutMs,
  );
  await openRemoteImport(page);
  await selectSession(page, sessionId);
  await page.locator('[data-deve-remote-import-apply-outcome="backend-typed"]').waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  await assertNoSensitiveProjection(page);
}

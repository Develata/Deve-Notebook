import assert from "node:assert/strict";

const timeoutMs = Number(process.env.DEVE_DESKTOP_PACKAGED_UI_TIMEOUT_MS ?? "60000");

export function delay(ms) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, ms));
}

export async function waitUntil(label, predicate, timeout = timeoutMs) {
  const started = Date.now();
  let lastError;
  while (Date.now() - started < timeout) {
    try {
      const value = await predicate();
      if (value) return value;
    } catch (error) {
      lastError = error;
    }
    await delay(250);
  }
  throw new Error(`timeout waiting for ${label}${lastError ? `: ${lastError.message}` : ""}`);
}

export async function findAppPage(browser) {
  return waitUntil("native WebView page", async () => {
    for (const context of browser.contexts()) {
      for (const page of context.pages()) {
        if (await page.locator("[data-deve-sync-status]").count()) return page;
      }
    }
    return null;
  });
}

export async function findBundledLocalPage(browser) {
  return waitUntil("bundled LocalBackend WebView page", async () => {
    for (const context of browser.contexts()) {
      for (const page of context.pages()) {
        const url = new URL(page.url());
        const bundled = (url.protocol === "http:" && url.hostname === "tauri.localhost" && !url.port)
          || (url.protocol === "tauri:" && url.hostname === "localhost" && !url.port);
        if (bundled && await page.locator("[data-deve-sync-status]").count()) return page;
      }
    }
    return null;
  });
}

export async function waitForReady(page) {
  await page.waitForFunction(
    () => document.querySelector("[data-deve-sync-status]")?.getAttribute("data-deve-sync-status") === "ready",
    null,
    { timeout: timeoutMs },
  );
}

export async function waitForWritableEditor(page) {
  const host = page.locator("[data-deve-editor-host=true]:visible").first();
  await host.waitFor({ state: "visible", timeout: timeoutMs });
  await waitUntil(
    "visible writable editor",
    async () => (await host.getAttribute("data-deve-editor-readonly")) === "false",
  );
}

export async function createAndEditDocument(page, path, content) {
  await page.locator("[data-deve-new-doc-button=true]").click({ force: true });
  const input = page.locator("[data-deve-search-input=true]");
  await input.waitFor({ state: "visible", timeout: timeoutMs });
  await input.fill(`+${path}`);
  await page.locator('[data-deve-search-result-action="create-doc"]').first().click();
  await waitForWritableEditor(page);
  await waitUntil("editor bridge content API", () => page.evaluate(
    () => typeof window.getEditorContent === "function"
      && typeof window.getEditorContent() === "string",
  ));
  const cm = page.locator(".cm-content:visible").first();
  await cm.click({ force: true });
  await cm.pressSequentially(content, { delay: 2 });
  await page.waitForFunction(
    (expected) => typeof window.getEditorContent === "function"
      && window.getEditorContent()?.includes(expected),
    content,
    { timeout: timeoutMs },
  );
  await page.waitForFunction(
    () => document.querySelector("[data-deve-pending-ack-count]")
      ?.getAttribute("data-deve-pending-ack-count") === "0",
    null,
    { timeout: timeoutMs },
  );
}

export async function commitAndVerifyHistory(page, message) {
  await page.locator("[data-deve-activity-more-button]").click();
  await page.locator('[data-deve-activity-more-item="activity_more_item_source_control"]').click();
  const textarea = page.locator('textarea[name="commit-message"]');
  await textarea.waitFor({ state: "visible", timeout: timeoutMs });
  await waitUntil("commit message input enabled", () => textarea.isEnabled());
  await textarea.fill(message);
  const commitPanel = textarea.locator("xpath=ancestor::div[contains(@class,'border-t')][1]");
  const commitButton = commitPanel.locator("button:has(.codicon-check)").first();
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

async function readScope(page) {
  return page.locator("[data-deve-sync-status]").first().evaluate((element) => ({
    status: element.getAttribute("data-deve-sync-status"),
    repoId: element.getAttribute("data-deve-repo-id") ?? "",
    scopeNonce: Number(element.getAttribute("data-deve-scope-nonce")),
  }));
}

async function openRepoSwitcher(page) {
  const trigger = page.locator("[data-deve-repo-switcher-trigger]").first();
  await trigger.waitFor({ state: "visible", timeout: timeoutMs });
  await trigger.click();
  await page.locator("[data-deve-repo-switcher-create]").waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
}

export async function exerciseLastRepoRemoval(page) {
  const before = await readScope(page);
  assert.equal(before.status, "ready");
  assert.ok(before.repoId, "repo removal journey requires a selected repo");
  assert.ok(Number.isInteger(before.scopeNonce) && before.scopeNonce > 0);

  await openRepoSwitcher(page);
  const rows = page.locator("[data-deve-repo-switcher-item]");
  assert.equal(await rows.count(), 1, "repo removal journey requires exactly one local repo");
  const alias = (await rows.first().getAttribute("data-deve-repo-switcher-item-name")) ?? "";
  assert.ok(alias, "last repo must expose a backend-projected alias");
  const row = rows.first().locator("xpath=..");
  await row.locator("[data-deve-repo-switcher-actions]").click();
  await page.locator("[data-deve-repo-switcher-remove]").click();

  const dialog = page.locator('[data-deve-repo-removal-dialog="visible"]');
  await dialog.waitFor({ state: "visible", timeout: timeoutMs });
  await dialog.locator("#repo-removal-preserved-heading").waitFor({
    state: "visible",
    timeout: timeoutMs,
  });
  const confirm = dialog.locator('[data-deve-repo-removal-confirm="true"]');
  assert.equal(await confirm.isEnabled(), true, "backend preview must admit the removal");
  await confirm.click();
  await dialog.waitFor({ state: "hidden", timeout: timeoutMs });

  const noScope = await waitUntil("last repo removal NoScope finalization", async () => {
    const current = await readScope(page);
    return current.repoId === "" ? current : null;
  });
  assert.ok(
    Number.isInteger(noScope.scopeNonce) && noScope.scopeNonce > before.scopeNonce,
    "last repo removal must advance the backend scope nonce",
  );
  await openRepoSwitcher(page);
  assert.equal(await page.locator("[data-deve-repo-switcher-item]").count(), 0);
  await page.locator("[data-deve-repo-switcher-backdrop]").click();
  return {
    alias,
    removedRepoId: before.repoId,
    scopeNonceBeforeRemoval: before.scopeNonce,
    scopeNonceAfterRemoval: noScope.scopeNonce,
    noScope: true,
  };
}

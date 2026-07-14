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

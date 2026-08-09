import { clickWebViewPoint } from "./android-webview-pointer.mjs";
import {
  closeMobileSidebar,
  openMobileSidebarView,
  readSourceControlCommitState,
  sourceControlCommitAcknowledged,
  sourceControlCommitReady,
  typeAndroidTextField,
} from "./mobile-webview-interaction.mjs";

async function openSourceControl(page, { click, waitUntil }) {
  const mobile = await page.call(() =>
    Boolean(globalThis.__deveVisibleElement('[data-deve-layout-mode="mobile"]')));
  if (mobile) {
    await openMobileSidebarView(page, "source_control", { click, waitUntil });
    return;
  }
  await click(page, "[data-deve-activity-more-button]");
  await waitUntil("Source Control menu item", () => page.call(() =>
    Boolean(globalThis.__deveVisibleElement(
      '[data-deve-activity-more-item="activity_more_item_source_control"]'))));
  await click(page, '[data-deve-activity-more-item="activity_more_item_source_control"]');
}

export async function commitSourceControlChange(
  page,
  message,
  { click, waitUntil, delay, inputText },
) {
  await openSourceControl(page, { click, waitUntil });
  const textarea = 'textarea[name="commit-message"]';
  await waitUntil("commit message input", () => page.call((selector) =>
    Boolean(globalThis.__deveVisibleElement(selector)), textarea));
  await waitUntil("commit input enabled", () => page.call((selector) =>
    !globalThis.__deveVisibleElement(selector)?.disabled, textarea));
  await waitUntil("confirmed changes loaded", async () => {
    const state = await readSourceControlCommitState(page);
    return state.confirmedCount > 0 ? state : null;
  }).catch(async (error) => {
    const diagnostics = await page.call(() => ({
      commit: (() => {
        const field = globalThis.__deveVisibleElement('textarea[name="commit-message"]');
        const panel = field?.closest("div.border-t");
        const button = panel?.querySelector("button:has(.codicon-check)");
        return {
          message: field?.value ?? null,
          fieldDisabled: field?.disabled ?? null,
          buttonDisabled: button?.disabled ?? null,
          buttonTitle: button?.title ?? null,
        };
      })(),
      sections: [...document.querySelectorAll("[data-deve-sc-section-body]")].map((section) => ({
        kind: section.getAttribute("data-deve-sc-section-body"),
        rows: [...section.querySelectorAll(
          '[data-deve-mobile-touch-target="source-control-change-row"]',
        )].map((row) => ({
          status: row.querySelector("[data-deve-sc-status-kind]")
            ?.getAttribute("data-deve-sc-status-kind") ?? null,
          text: row.textContent?.replace(/\s+/g, " ").trim().slice(0, 160) ?? null,
        })),
        text: section.textContent?.replace(/\s+/g, " ").trim().slice(0, 300) ?? "",
      })),
      syncStatus: document.querySelector("[data-deve-sync-status]")
        ?.getAttribute("data-deve-sync-status") ?? null,
      syncBanner: document.querySelector("[data-deve-sync-banner]")
        ?.textContent?.replace(/\s+/g, " ").trim().slice(0, 500) ?? null,
      editorOpenRequestId: globalThis.__deveVisibleElement("[data-deve-editor-host=true]")
        ?.getAttribute("data-deve-editor-open-request-id") ?? null,
      editorContent: window.getEditorContent?.() ?? null,
      body: document.body?.textContent?.replace(/\s+/g, " ").trim().slice(0, 1000) ?? null,
    }));
    throw new Error(`${error.message}; sourceControl=${JSON.stringify(diagnostics)}`);
  });
  await typeAndroidTextField(page, textarea, message, {
    tap: async (point) => clickWebViewPoint(page, point),
    delay,
    inputText,
  });
  await waitUntil("commit message binding", () => page.call((expected) =>
    globalThis.__deveVisibleElement('textarea[name="commit-message"]')?.value === expected,
  message));
  await waitUntil("commit enabled", async () =>
    sourceControlCommitReady(await readSourceControlCommitState(page), message)).catch(async (error) => {
    const diagnostics = await readSourceControlCommitState(page);
    throw new Error(`${error.message}; commit=${JSON.stringify(diagnostics)}`);
  });
  const committed = await page.call(() => {
    const field = globalThis.__deveVisibleElement('textarea[name="commit-message"]');
    const button = field?.closest("div.border-t")?.querySelector("button:has(.codicon-check)");
    if (!button) return false;
    button.click();
    return true;
  });
  if (!committed) throw new Error("commit action not found");
  await waitUntil("commit acknowledgement refresh", async () =>
    sourceControlCommitAcknowledged(await readSourceControlCommitState(page)), 120000).catch(
    async (error) => {
      const diagnostics = await readSourceControlCommitState(page);
      throw new Error(`${error.message}; commit=${JSON.stringify(diagnostics)}`);
    },
  );
  const historyToggle = '[data-deve-sc-panel-toggle="history"]';
  const historyExpanded = await page.call((selector) =>
    document.querySelector(selector)?.getAttribute("aria-expanded") === "true", historyToggle);
  if (!historyExpanded) await click(page, historyToggle);
  await waitUntil("commit visible in history", () => page.call((expected) => {
    const history = globalThis.__deveVisibleElement('[data-deve-sc-panel-body="history"]');
    return history?.textContent?.includes(expected) ?? false;
  }, message), 120000).catch(async (error) => {
    const diagnostics = await page.call(() => ({
      historyText: globalThis.__deveVisibleElement('[data-deve-sc-panel-body="history"]')
        ?.textContent?.slice(0, 500) ?? null,
      confirmedRows: document
        .querySelectorAll(
          '[data-deve-sc-section-body="confirmed-ledger"] '
            + '[data-deve-mobile-touch-target="source-control-change-row"]',
        ).length,
      pending: document.querySelector("[data-deve-mobile-pending-ack-count]")
        ?.getAttribute("data-deve-mobile-pending-ack-count") ?? null,
    }));
    throw new Error(`${error.message}; history=${JSON.stringify(diagnostics)}`);
  });
  await closeMobileSidebar(page, { click, waitUntil });
}

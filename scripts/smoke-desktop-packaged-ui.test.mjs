import assert from "node:assert/strict";
import fs from "node:fs";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { isDirectInvocation } from "./smoke-desktop-packaged-ui.mjs";

const scriptUrl = new URL("./smoke-desktop-packaged-ui.mjs", import.meta.url);
const source = fs.readFileSync(scriptUrl, "utf8");
const businessFlowSource = fs.readFileSync(
  new URL("./lib/desktop-webview-business-flow.mjs", import.meta.url),
  "utf8",
);

test("packaged smoke uses real UI intents for document and source-control flows", () => {
  assert.match(businessFlowSource, /data-deve-search-result-action=\\?"create-doc/);
  assert.match(businessFlowSource, /cm\.click\(\{ force: true \}\)/);
  assert.match(businessFlowSource, /cm\.pressSequentially\(content/);
  assert.match(businessFlowSource, /textarea\[name=\\?"commit-message\\?"\]/);
  assert.match(businessFlowSource, /data-deve-sc-panel-body=\\?"history\\?"/);
  assert.match(businessFlowSource, /data-deve-repo-switcher-remove/);
  assert.match(businessFlowSource, /initial zero-repo BootstrapUnbound/);
  assert.match(businessFlowSource, /current\.scopeNonce === 0/);
  assert.match(businessFlowSource, /activity_more_item_explorer/);
  assert.match(businessFlowSource, /data-deve-repo-switcher-trigger]:visible/);
  assert.match(businessFlowSource, /fresh LocalBackend must not auto-create a default repo/);
  assert.match(businessFlowSource, /first Create must advance the backend scope nonce/);
  assert.match(source, /createFirstRepoFromBootstrapUnbound/);
  assert.match(source, /firstRepoCreate/);
  assert.match(businessFlowSource, /data-deve-repo-removal-confirm/);
  assert.match(businessFlowSource, /last repo removal NoScope finalization/);
  assert.match(businessFlowSource, /noScope\.scopeNonce > before\.scopeNonce/);
  assert.match(source, /exerciseLastRepoRemoval/);
  assert.match(source, /repoRemovalNoScope/);
  assert.doesNotMatch(businessFlowSource, /fetch\([^)]*(ledger|commit|source.control)/i);
});

test("packaged smoke checks modal focus trap and Escape", () => {
  assert.match(source, /data-deve-open-search-button=true/);
  assert.match(source, /query\.fill\(">settings"\)/);
  assert.match(source, /data-deve-search-result-action="run-command/);
  assert.match(source, /aria-modal/);
  assert.match(source, /Tab from the last control must wrap to the first/);
  assert.match(source, /Shift\+Tab from the first control must wrap to the last/);
  assert.match(source, /keyboard\.press\("Escape"\)/);
});

test("direct invocation guard recognizes the local script", () => {
  assert.equal(isDirectInvocation(fileURLToPath(scriptUrl), scriptUrl.href), true);
});

test("direct execution fails closed without a CDP endpoint", () => {
  const work = mkdtempSync(join(tmpdir(), "deve-packaged-ui-direct-"));
  try {
    const result = spawnSync(process.execPath, [fileURLToPath(scriptUrl)], {
      cwd: work,
      env: { ...process.env, DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT: "" },
      encoding: "utf8",
    });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT is required/);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
});

import assert from "node:assert/strict";
import fs, { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  editorContentIncludes,
  isDirectInvocation,
  pendingAckCount,
  renderedShellPresent,
  renderedShellSelector,
  relevantConsoleErrors,
  webSocketMatchesExpectedOrigin,
  waitForRenderedShell,
} from "./smoke-docker-multiclient.mjs";
import {
  selectWorkspaceRoot,
  validateWorkspaceIdentity,
} from "./lib/docker-multiclient-product-journeys.mjs";

test("pending marker parser rejects malformed counts", async () => {
  const page = {
    locator() {
      return {
        first() {
          return {
            evaluate(callback) {
              return callback({ getAttribute: () => "not-a-count" });
            },
          };
        },
      };
    },
  };
  await assert.rejects(() => pendingAckCount(page), /invalid pending ack count marker/);
});

test("rendered shell requires a login or sync marker", () => {
  const emptyRoot = { querySelector: () => null };
  const loginRoot = {
    querySelector: (selector) => selector === renderedShellSelector ? {} : null,
  };

  assert.equal(renderedShellPresent(renderedShellSelector, emptyRoot), false);
  assert.equal(renderedShellPresent(renderedShellSelector, loginRoot), true);
});

test("render wait forwards the marker and timeout to the browser", async () => {
  const calls = [];
  const page = {
    async waitForFunction(predicate, selector, options) {
      calls.push({ predicate, selector, options });
    },
  };

  await waitForRenderedShell(page, 4321);

  assert.equal(calls.length, 1);
  assert.equal(calls[0].predicate, renderedShellPresent);
  assert.equal(calls[0].selector, renderedShellSelector);
  assert.deepEqual(calls[0].options, { timeout: 4321 });
});

test("editor content wait treats an unready bridge as pending", () => {
  assert.equal(editorContentIncludes("ready", {}), false);
  assert.equal(editorContentIncludes("ready", { getEditorContent: () => null }), false);
  assert.equal(
    editorContentIncludes("ready", { getEditorContent: () => "eventually ready" }),
    true,
  );
});

test("websocket proof requires the expected origin and relative ws path", () => {
  assert.equal(
    webSocketMatchesExpectedOrigin("ws://127.0.0.1:3101/ws", "http://127.0.0.1:3101"),
    true,
  );
  assert.equal(
    webSocketMatchesExpectedOrigin("ws://elsewhere.invalid/ws", "http://127.0.0.1:3101"),
    false,
  );
  assert.equal(
    webSocketMatchesExpectedOrigin("ws://127.0.0.1:3101/other", "http://127.0.0.1:3101"),
    false,
  );
});

test("internet disconnected console errors are ignored only during the offline window", () => {
  const message = "Failed to load resource: net::ERR_INTERNET_DISCONNECTED";
  assert.deepEqual(
    relevantConsoleErrors({ consoleErrors: [{ message, duringOffline: true }] }),
    [],
  );
  assert.deepEqual(
    relevantConsoleErrors({ consoleErrors: [{ message, duringOffline: false }] }),
    [{ message, duringOffline: false }],
  );
});

test("login checks page health only after the rendered shell wait", () => {
  const source = fs.readFileSync(new URL("./smoke-docker-multiclient.mjs", import.meta.url), "utf8");
  assert.match(
    source,
    /async function login\(page, diag\) \{[\s\S]*?await page\.goto[\s\S]*?await waitForRenderedShell\(page, timeoutMs\);[\s\S]*?await assertPageHealthy\(page, diag\);/,
  );
});

test("offline recovery attempts input before asserting local pending and peer immutability", () => {
  const source = fs.readFileSync(new URL("./smoke-docker-multiclient.mjs", import.meta.url), "utf8");
  assert.match(
    source,
    /async function exerciseOfflineRecovery[\s\S]*?keyboard\.type\(blockedInput\)[\s\S]*?offline input must not change local editor content[\s\S]*?offline input must not enqueue pending edits[\s\S]*?offline input must not reach the peer editor[\s\S]*?setOffline\(false\)/,
  );
});

test("playwright bootstrap validates module resolution instead of a stale directory", () => {
  const source = fs.readFileSync(new URL("./smoke-docker-multiclient.sh", import.meta.url), "utf8");

  assert.match(source, /createRequire\(process\.env\.DEVE_DOCKER_MULTI_PLAYWRIGHT_REQUIRE_FROM\)/);
  assert.match(source, /typeof playwright\.chromium\?\.launch !== "function"/);
  assert.doesNotMatch(source, /\[\[ ! -d "\$PLAYWRIGHT_WORK_DIR\/node_modules\/playwright" \]\]/);
});

test("direct invocation recognizes Windows and Git Bash path forms", () => {
  const scriptUrl = new URL("./smoke-docker-multiclient.mjs", import.meta.url);
  const scriptPath = fileURLToPath(scriptUrl);

  assert.equal(isDirectInvocation(scriptPath, scriptUrl.href), true);
  assert.equal(isDirectInvocation(scriptPath.replaceAll("\\", "/"), scriptUrl.href), true);
  assert.equal(isDirectInvocation(undefined, scriptUrl.href), false);
});

test("direct execution enters main instead of silently succeeding", () => {
  const scriptUrl = new URL("./smoke-docker-multiclient.mjs", import.meta.url);
  const tempDir = mkdtempSync(join(tmpdir(), "deve-multiclient-direct-"));
  try {
    const result = spawnSync(process.execPath, [fileURLToPath(scriptUrl)], {
      cwd: tempDir,
      env: {
        ...process.env,
        DEVE_DOCKER_MULTI_PLAYWRIGHT_REQUIRE_FROM: join(tempDir, "package.json"),
      },
      encoding: "utf8",
    });

    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /docker-multiclient-smoke: Playwright failure/);
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test("product journey accepts one canonical projection workspace only", () => {
  const bareLocator = `version = 2

[[locators]]
repo_id = "11111111-1111-4111-8111-111111111111"
workspace_segment = "11111111-1111-4111-8111-111111111111"
projection_base_abs = "/notes"
canonicalized_at_unix_ms = 1
`;
  assert.equal(
    selectWorkspaceRoot(
      bareLocator,
      "11111111-1111-4111-8111-111111111111",
    ),
    "/notes/11111111-1111-4111-8111-111111111111",
  );
  const aliasLocator = `version = 2

[[locators]]
repo_id = '11111111-1111-4111-8111-111111111111'
workspace_segment = 'one--11111111-1111-4111-8111-111111111111'
projection_base_abs = '/notes'
canonicalized_at_unix_ms = 1

[[locators]]
repo_id = '22222222-2222-4222-8222-222222222222'
workspace_segment = 'two--22222222-2222-4222-8222-222222222222'
projection_base_abs = '/notes'
canonicalized_at_unix_ms = 1
`;
  assert.equal(
    selectWorkspaceRoot(
      aliasLocator,
      "22222222-2222-4222-8222-222222222222",
    ),
    "/notes/two--22222222-2222-4222-8222-222222222222",
  );
  assert.throws(() => selectWorkspaceRoot(
    bareLocator.replace('projection_base_abs = "/notes"', 'projection_base_abs = "/tmp"'),
    "11111111-1111-4111-8111-111111111111",
  ));
  validateWorkspaceIdentity(
    'version = 1\nrepo_id = "11111111-1111-4111-8111-111111111111"\nrepo_name = "machine"\n',
    "11111111-1111-4111-8111-111111111111",
  );
  assert.throws(() => validateWorkspaceIdentity(
    'version = 1\nrepo_id = "22222222-2222-4222-8222-222222222222"\nrepo_name = "machine"\n',
    "11111111-1111-4111-8111-111111111111",
  ));
});

test("tag-ready product journey covers repo, typed diff, source control, and external apply", () => {
  const source = fs.readFileSync(
    new URL("./lib/docker-multiclient-product-journeys.mjs", import.meta.url),
    "utf8",
  );
  assert.match(source, /data-deve-repo-switcher-create-input/);
  assert.match(source, /data-deve-diff-projection=\\?"backend-typed/);
  assert.match(source, /textarea\[name=\\?"commit-message/);
  assert.match(source, /data-deve-external-section-body=\\?"pending/);
  assert.match(source, /data-deve-external-apply=\\?"true/);
  assert.match(source, /external workspace mutation must not bypass ledger authority/);
});

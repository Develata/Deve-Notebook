import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { closeBrowserResources } from "./lib/docker-remote-import-browser-cleanup.mjs";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

test("compose pins provider manifests and keeps credentials in environment", () => {
  const compose = read("docker-compose.remote-import.yml");
  assert.match(compose, /rclone\/rclone@sha256:[0-9a-f]{64}/u);
  assert.match(compose, /minio\/minio@sha256:[0-9a-f]{64}/u);
  assert.match(compose, /minio\/mc@sha256:[0-9a-f]{64}/u);
  assert.match(compose, /DEVE_REMOTE_IMPORT_S3_SECRET_ACCESS_KEY/u);
  assert.match(compose, /DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_EDGE_IP/u);
  assert.match(compose, /DEVE_REMOTE_IMPORT_WEBDAV_EDGE_IP/u);
  assert.match(compose, /DEVE_REMOTE_IMPORT_S3_EDGE_IP/u);
  assert.match(compose, /\n  webdav-failure:/u);
  assert.match(compose, /\n  deve-webdav-failure:/u);
  assert.doesNotMatch(compose, /AKIA[0-9A-Z]{16}/u);
  assert.doesNotMatch(compose, /minioadmin/u);
});

test("producer fixes every ambient environment input that can alter B6", () => {
  const registry = JSON.parse(read("docs/registry/acceptance-producers.json"));
  const producer = registry.producers.find(
    ({ producer_id: id }) => id === "docker.remote-import-browser",
  );
  assert.deepEqual(producer.environment, {
    DEVE_REMOTE_IMPORT_BROWSER_CLOSE_TIMEOUT_MS: "15000",
    DEVE_REMOTE_IMPORT_CHROME_CHECKPOINT: "0",
    DEVE_REMOTE_IMPORT_CHROME_WAIT_SECONDS: "600",
    DEVE_REMOTE_IMPORT_COMPOSE_FILE: "docker-compose.remote-import.yml",
    DEVE_REMOTE_IMPORT_DOCKER_BIN: "docker",
    DEVE_REMOTE_IMPORT_HEADLESS: "1",
    DEVE_REMOTE_IMPORT_PLAYWRIGHT_PACKAGE: "playwright@1.55.0",
    DEVE_REMOTE_IMPORT_TIMEOUT_MS: "90000",
  });
});

test("fixture lifecycle is explicit and browser contexts close in finally", () => {
  const shell = read("scripts/smoke-docker-remote-import.sh");
  const shellTest = read("scripts/smoke-docker-remote-import.test.sh");
  const lifecycle = read("scripts/lib/docker-remote-import-fixture.sh");
  const edge = read("scripts/lib/docker-remote-import-edge.sh");
  const stableEdge = read("scripts/lib/docker-remote-import-stable-edge.sh");
  const browser = read("scripts/smoke-docker-remote-import.mjs");
  const journeys = read("scripts/lib/docker-remote-import-journeys.mjs");
  const browserCleanup = read(
    "scripts/lib/docker-remote-import-browser-cleanup.mjs",
  );
  const chromeCheckpoint = read(
    "scripts/lib/docker-remote-import-chrome-checkpoint.sh",
  );
  assert.match(shell, /trap cleanup_on_exit EXIT/u);
  assert.ok(
    shell.indexOf('node --test "$ROOT_DIR/scripts/smoke-docker-remote-import.test.mjs"') <
      shell.indexOf("remote_import_fixture_start_tunnel webdav_failure"),
  );
  assert.ok(
    shell.indexOf('bash "$ROOT_DIR/scripts/smoke-docker-remote-import.test.sh"') <
      shell.indexOf("remote_import_fixture_start_tunnel webdav_failure"),
  );
  assert.match(
    shellTest,
    /unset DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_TUNNEL_PID[\s\S]*unset DEVE_REMOTE_IMPORT_S3_TUNNEL_TOKEN/u,
  );
  assert.match(shell, /sealed-before-refresh/u);
  assert.match(shell, /backend-owned-diff/u);
  assert.match(lifecycle, /down --timeout 5 -v --remove-orphans/u);
  assert.match(lifecycle, /docker-remote-import-absence\.sh/u);
  assert.match(edge, /cloudflare-dns\.com/u);
  assert.match(edge, /origin\.hostname, "trycloudflare\.com"/u);
  assert.match(edge, /--entrypoint curl/u);
  assert.match(edge, /--request "\$method"/u);
  assert.match(edge, /--http1\.1/u);
  assert.match(edge, /--resolve "\$host:443:\$candidate_ip"/u);
  assert.match(edge, /probe_attempt in 1 2 3 4 5/u);
  assert.match(edge, /DEVE_REMOTE_IMPORT_EDGE_PROPAGATION_WINDOW_SECS/u);
  assert.match(edge, /remote_import_edge_is_excluded/u);
  assert.match(stableEdge, /for edge_attempt in 1 2/u);
  assert.match(stableEdge, /--force-recreate/u);
  assert.match(stableEdge, /240 60 "\$method"/u);
  assert.match(stableEdge, /verify_retryable_edge_failure/u);
  assert.match(stableEdge, /State\.Running/u);
  assert.match(edge, /waiting for %s tunnel edge route propagation \(sweep %s\)/u);
  assert.ok(
    edge.includes('[[ "$status" =~ ^2[0-9][0-9]$ ]]'),
    "edge probe acceptance must stay pinned to exact-candidate 2xx",
  );
  assert.match(edge, /Content-Type: application\/xml; charset=utf-8/u);
  assert.match(edge, /d:resourcetype/u);
  assert.match(lifecycle, /Content-Type: application\/xml; charset=utf-8/u);
  assert.match(lifecycle, /d:resourcetype/u);
  assert.match(lifecycle, /--http1\.1/u);
  assert.match(edge, /container ls -aq/u);
  assert.match(lifecycle, /--protocol http2/u);
  assert.match(lifecycle, /consecutive_successes=0/u);
  assert.match(shell, /admit_stable_edge deve-webdav webdav[\s\S]*PROPFIND/u);
  assert.match(shell, /admit_stable_edge deve-s3 s3[\s\S]*GET/u);
  assert.match(browser, /finally \{/u);
  assert.match(browser, /closeBrowserResources\(context, browser\)/u);
  assert.match(browserCleanup, /resource\.close\(\)/u);
  assert.match(browserCleanup, /\["Playwright context", context\]/u);
  assert.match(browserCleanup, /\["Playwright browser", browser\]/u);
  assert.match(journeys, /exerciseWebDavFailure/u);
  assert.match(journeys, /provider failure must not install a Ready selection/u);
  assert.match(journeys, /healthy WebDAV provider target must exist before Prepare/u);
  assert.match(journeys, /prepareWebDavWithBoundedRetry/u);
  assert.match(journeys, /refreshRemoteImport/u);
  assert.match(journeys, /attempt <= 5/u);
  assert.match(journeys, /failure must not install a Ready selection/u);
  assert.match(journeys, /must persist exactly one new failed session/u);
  assert.match(journeys, /failure discarded/u);
  const failureRun =
    'node "$ROOT_DIR/scripts/smoke-docker-remote-import.mjs" webdav-failure\n';
  const healthyRun =
    'node "$ROOT_DIR/scripts/smoke-docker-remote-import.mjs" webdav\n';
  assert.ok(
    shell.indexOf(failureRun) < shell.indexOf(healthyRun),
    "isolated provider failure must complete before the healthy WebDAV journey",
  );
  assert.match(shell, /remote_import_chrome_checkpoint_wait/u);
  assert.ok(
    shell.indexOf("remote_import_fixture_start_tunnel s3") <
      shell.indexOf('smoke-docker-remote-import.mjs" webdav'),
    "S3 tunnel must start before the WebDAV journey to absorb DNS propagation",
  );
  assert.ok(
    shell.indexOf("playwright install chromium") <
      shell.indexOf("remote_import_fixture_start_tunnel webdav_failure"),
    "browser tooling must be ready before ephemeral quick tunnels start",
  );
  assert.match(chromeCheckpoint, /DEVE_REMOTE_IMPORT_CHROME_CHECKPOINT:-0/u);
  assert.match(chromeCheckpoint, /fs\.renameSync\(temporary, target\)/u);
  assert.doesNotMatch(chromeCheckpoint, /S3_SECRET|ACCESS_KEY/u);
});

test("browser orchestration never uses a command-string shell", () => {
  const runtime = read("scripts/lib/docker-remote-import-runtime.mjs");
  assert.match(runtime, /spawnSync/u);
  assert.match(runtime, /shell: false/u);
  assert.match(runtime, /activity_more_item_explorer/u);
  assert.doesNotMatch(runtime, /execSync|execFileSync/u);
});

test("browser cleanup remains best-effort and fails closed", async () => {
  const closed = [];
  const context = {
    close: async () => {
      closed.push("context");
      throw new Error("context close rejected");
    },
  };
  const browser = {
    close: async () => {
      closed.push("browser");
      throw new Error("browser close rejected");
    },
  };
  await assert.rejects(
    closeBrowserResources(context, browser, 50),
    (error) => error instanceof AggregateError && error.errors.length === 2,
  );
  assert.deepEqual(closed, ["context", "browser"]);
});

test("browser cleanup timeout does not skip browser close", async () => {
  let browserClosed = false;
  await assert.rejects(
    closeBrowserResources(
      { close: () => new Promise(() => {}) },
      {
        close: async () => {
          browserClosed = true;
        },
      },
      5,
    ),
    (error) =>
      error instanceof AggregateError &&
      error.errors.some((failure) => /timed out/u.test(failure.message)),
  );
  assert.equal(browserClosed, true);
});

test("new handwritten files remain below the hard fuse", () => {
  for (const path of [
    "scripts/smoke-docker-remote-import.sh",
    "scripts/smoke-docker-remote-import.mjs",
    "scripts/lib/docker-remote-import-browser-cleanup.mjs",
    "scripts/lib/docker-remote-import-chrome-checkpoint.sh",
    "scripts/lib/docker-remote-import-edge.sh",
    "scripts/lib/docker-remote-import-stable-edge.sh",
    "scripts/lib/docker-remote-import-absence.sh",
    "scripts/lib/docker-remote-import-fixture.sh",
    "scripts/lib/docker-remote-import-runtime.mjs",
    "scripts/lib/docker-remote-import-journeys.mjs",
  ]) {
    const lines = read(path).split(/\r?\n/u).length;
    assert.ok(lines <= 500, `${path} has ${lines} lines`);
  }
});

import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

test("tag-ready product journey covers destructive repo removal, typed diff, source control, and external apply", () => {
  const source = fs.readFileSync(
    new URL("./lib/docker-multiclient-product-journeys.mjs", import.meta.url),
    "utf8",
  );
  assert.match(source, /data-deve-repo-switcher-create-input/);
  assert.match(source, /data-deve-repo-switcher-remove/);
  assert.match(source, /data-deve-repo-removal-confirm/);
  assert.match(source, /fallback repository ready after removal/);
  assert.match(source, /assertRemovalPreservation/);
  assert.match(source, /sha256sum/);
  assert.match(source, /test ! -e "\$root\/\.notegit"/);
  assert.match(source, /test -f "\$root\/\.git\/config"/);
  assert.match(source, /data-deve-diff-projection=\\?"backend-typed/);
  assert.match(source, /textarea\[name=\\?"commit-message/);
  assert.match(source, /data-deve-external-section-body=\\?"pending/);
  assert.match(source, /data-deve-external-apply=\\?"true/);
  assert.match(source, /external workspace mutation must not bypass ledger authority/);
});

test("tag-ready product journey covers mobile last-repo NoScope, restart, and first create", () => {
  const journey = fs.readFileSync(
    new URL("./lib/docker-multiclient-product-journeys.mjs", import.meta.url),
    "utf8",
  );
  const runner = fs.readFileSync(
    new URL("./smoke-docker-multiclient.mjs", import.meta.url),
    "utf8",
  );
  assert.match(journey, /exerciseLastRepoNoScope/);
  assert.match(journey, /repo_creation_projection_base/);
  assert.match(journey, /assertNoScope/);
  assert.match(journey, /restartCandidateContainer/);
  assert.match(journey, /createFirstRepoFromNoScope/);
  assert.match(journey, /scrollWidth <= overflow\.width/);
  assert.match(runner, /viewport: \{ width: 390, height: 844 \}/);
  assert.match(runner, /await reopenNoScope\(pageA, diagA\)/);
  assert.match(runner, /await reopenNoScope\(mobilePage, mobileDiag\)/);
  assert.match(runner, /runtime_incarnation/);
  assert.match(runner, /some\(\(\{ url, frames \}\) => webSocketMatchesExpectedOrigin\(url\) && frames > 0\)/);
  assert.match(runner, /page\.goto\("about:blank"\)/);
  assert.match(runner, /mobileViewport: mobilePage\.viewportSize\(\)/);
  assert.match(runner, /mobileServerFrames: mobileDiag\.sockets/);
  assert.match(runner, /createFirstRepoFromNoScope\(mobilePage, \[pageA, pageB\]\)/);
  assert.match(journey, /created repo scope in observer/);
  assert.ok(
    [...runner.matchAll(/assertRemovalPreservation\(/gu)].length >= 2,
    "the old workspace must be rechecked after restart and after same-host recreation",
  );
});

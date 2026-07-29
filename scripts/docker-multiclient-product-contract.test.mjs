import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import {
  selectWorkspaceRoot,
  validateWorkspaceIdentity,
} from "./lib/docker-multiclient-product-journeys.mjs";

test("tag-ready product journey covers destructive repo removal, typed diff, source control, and external apply", () => {
  const lifecycle = fs.readFileSync(
    new URL("./lib/docker-multiclient-repo-lifecycle.mjs", import.meta.url),
    "utf8",
  );
  const workspace = fs.readFileSync(
    new URL("./lib/docker-multiclient-workspace.mjs", import.meta.url),
    "utf8",
  );
  const sourceControl = fs.readFileSync(
    new URL("./lib/docker-multiclient-source-control.mjs", import.meta.url),
    "utf8",
  );
  assert.match(lifecycle, /data-deve-repo-switcher-create-input/);
  assert.match(lifecycle, /data-deve-repo-switcher-remove/);
  assert.match(lifecycle, /data-deve-repo-removal-confirm/);
  assert.match(lifecycle, /fallback repository ready after removal/);
  assert.match(lifecycle, /assertRemovalPreservation/);
  assert.match(workspace, /sha256sum/);
  assert.match(workspace, /test ! -e "\$root\/\.notegit"/);
  assert.match(workspace, /test -f "\$root\/\.git\/config"/);
  assert.match(sourceControl, /data-deve-diff-projection=\\?"backend-typed/);
  assert.match(sourceControl, /textarea\[name=\\?"commit-message/);
  assert.match(sourceControl, /data-deve-external-section-body=\\?"pending/);
  assert.match(sourceControl, /data-deve-external-apply=\\?"true/);
  assert.match(sourceControl, /external workspace mutation must not bypass ledger authority/);
  assert.match(sourceControl, /first client editor settled before external mutation/);
  assert.match(sourceControl, /peer editor settled before external mutation/);
  assert.match(
    sourceControl,
    /settled before external mutation[\s\S]*?mutateWorkspaceFile\(repoId, path, externalContent\)/,
    "the authority baseline must settle before the external workspace mutation",
  );
});

test("tag-ready product journey covers mobile last-repo NoScope, restart, and first create", () => {
  const journey = fs.readFileSync(
    new URL("./lib/docker-multiclient-repo-lifecycle.mjs", import.meta.url),
    "utf8",
  );
  const workspace = fs.readFileSync(
    new URL("./lib/docker-multiclient-workspace.mjs", import.meta.url),
    "utf8",
  );
  const runner = fs.readFileSync(
    new URL("./smoke-docker-multiclient.mjs", import.meta.url),
    "utf8",
  );
  assert.match(journey, /exerciseLastRepoNoScope/);
  assert.match(workspace, /repo_creation_projection_base/);
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

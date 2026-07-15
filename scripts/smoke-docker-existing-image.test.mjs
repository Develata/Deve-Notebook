import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const release = fs.readFileSync(new URL("./smoke-docker-release.sh", import.meta.url), "utf8");
const multiclient = fs.readFileSync(
  new URL("./smoke-docker-multiclient.sh", import.meta.url),
  "utf8",
);
const compose = fs.readFileSync(
  new URL("../docker-compose.multiclient.yml", import.meta.url),
  "utf8",
);
const p2p = fs.readFileSync(new URL("./smoke-docker-p2p-mesh.sh", import.meta.url), "utf8");
const releaseWorkflow = fs.readFileSync(
  new URL("../.github/workflows/release.yml", import.meta.url),
  "utf8",
);

test("release existing-image mode validates the image and bypasses build", () => {
  assert.match(
    release,
    /if \[\[ "\$SKIP_BUILD"[\s\S]*?docker_cmd image inspect "\$IMAGE"[\s\S]*?else[\s\S]*?docker_cmd build -t "\$IMAGE"/,
  );
});

test("multiclient existing-image mode validates the image and prohibits compose build", () => {
  assert.match(
    multiclient,
    /if \[\[ "\$SKIP_BUILD"[\s\S]*?docker_cmd image inspect "\$IMAGE"[\s\S]*?docker_compose up -d --no-build[\s\S]*?else[\s\S]*?docker_compose up -d --build/,
  );
});

test("compose binds the explicitly selected candidate image", () => {
  assert.match(compose, /image: \$\{DEVE_DOCKER_MULTI_IMAGE:-deve-notebook:local-multiclient\}/);
  assert.match(multiclient, /DEVE_DOCKER_MULTI_IMAGE="\$IMAGE"/);
});

test("Docker acceptance smokes bind and revalidate one immutable candidate image ID", () => {
  for (const source of [multiclient, p2p]) {
    assert.match(source, /DEVE_RELEASE_CANDIDATE_IMAGE/);
    assert.match(source, /DEVE_RELEASE_CANDIDATE_IMAGE_ID/);
    assert.match(source, /docker_cmd image inspect --format '\{\{\.Id\}\}' "\$IMAGE"/);
    assert.match(source, /candidate image identity mismatch/);
  }
  assert.match(p2p, /DEVE_DOCKER_P2P_MESH_IMAGE="\$IMAGE"/);
});

test("release workflow validates the complete tag set before the first push", () => {
  const validation = releaseWorkflow.indexOf("validate-release-image-tags.sh");
  const firstPush = releaseWorkflow.indexOf('docker push "$tag"');
  assert.ok(validation >= 0, "tag-set validation marker must exist");
  assert.ok(firstPush > validation, "tag-set validation must run before any image push");
  assert.match(releaseWorkflow, /flavor: latest=false/);
  assert.match(releaseWorkflow, /latest_tag=.*GITHUB_OUTPUT[\s\S]*version_tag=.*GITHUB_OUTPUT/);
});

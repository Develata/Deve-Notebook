import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
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
const meshCompose = fs.readFileSync(
  new URL("../docker-compose.mesh.yml", import.meta.url),
  "utf8",
);
const meshBootstrap = fs.readFileSync(
  new URL("./docker-p2p-mesh-bootstrap.sh", import.meta.url),
  "utf8",
);
const p2p = fs.readFileSync(new URL("./smoke-docker-p2p-mesh.sh", import.meta.url), "utf8");
const releaseWorkflow = fs.readFileSync(
  new URL("../.github/workflows/release.yml", import.meta.url),
  "utf8",
);

function meshEvidenceCount(logs, peerId, repoId) {
  const start = p2p.indexOf("count_mesh_evidence_in_logs() {");
  const end = p2p.indexOf("server_peer_id_from_logs() {", start);
  assert.ok(start >= 0 && end > start, "mesh evidence parser function must exist");
  const python = process.env.DEVE_DOCKER_P2P_MESH_PYTHON_BIN
    ?? "python3";
  const parser = p2p.slice(start, end).replaceAll("\r\n", "\n");
  const command = `${parser}
count_mesh_evidence_in_logs "$MESH_LOGS" "$PEER_ID" "$REPO_ID"`;
  const forwarded = "MESH_LOGS:PEER_ID:PYTHON_BIN:REPO_ID";
  return spawnSync("bash", ["-s"], {
    encoding: "utf8",
    env: {
      ...process.env,
      MESH_LOGS: logs,
      PEER_ID: peerId,
      PYTHON_BIN: python,
      REPO_ID: repoId,
      WSLENV: process.env.WSLENV ? `${process.env.WSLENV}:${forwarded}` : forwarded,
    },
    input: command,
  });
}

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

test("P2P compose installs the shared repo key at the locator-owned workspace", () => {
  assert.match(meshCompose, /docker-p2p-mesh-bootstrap\.sh:ro/);
  assert.match(meshCompose, /command: \*deve-p2p-mesh-command/g);
  assert.match(meshBootstrap, /record_repo != repo_id/);
  assert.match(meshBootstrap, /matches != 1/);
  assert.match(meshBootstrap, /projection base mismatch/);
  assert.match(meshBootstrap, /workspace_segment" != "\."/);
  assert.match(meshBootstrap, /workspace identity mismatch/);
  assert.match(meshBootstrap, /existing repo key mismatch/);
  assert.match(meshBootstrap, /mktemp "\$key_dir\/\.repo\.key\.tmp\.XXXXXX"/);
  assert.match(meshBootstrap, /trap cleanup_key_tmp EXIT/);
  assert.match(meshBootstrap, /forward_signal TERM 143/);
  assert.match(p2p, /docker-p2p-mesh-bootstrap\.test\.sh/);
  assert.match(p2p, /docker-p2p-mesh-cleanup\.test\.sh/);
  assert.match(p2p, /receipt project\/state override rejected/);
  assert.doesNotMatch(meshCompose, /default--\$\$\{DEVE_DOCKER_P2P_MESH_REPO_ID\}/);
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

test("P2P readiness accepts both authenticated admissions without waiting for exchange close", () => {
  const waitStart = p2p.indexOf("wait_for_mesh_handshake() {");
  const countStart = p2p.indexOf("mesh_connection_count() {");
  assert.ok(waitStart >= 0 && countStart > waitStart, "mesh readiness functions must exist");
  const wait = p2p.slice(waitStart, countStart);
  assert.match(wait, /mesh_connection_count peer-a "\$PEER_B_EXPECTED_ID"/);
  assert.match(wait, /mesh_connection_count peer-b "\$PEER_A_EXPECTED_ID"/);
  assert.match(wait, /connections_a > 0 && connections_b > 0/);
  assert.doesNotMatch(wait, /grep -q "P2P mesh connector handshake completed"/);

  const countEnd = p2p.indexOf("server_peer_id_from_logs() {", countStart);
  const count = p2p.slice(countStart, countEnd);
  assert.match(count, /Session bound to peer/);
  assert.doesNotMatch(count, /Handling SyncHello from/);
});

test("P2P mesh evidence parser is identity-, repo-, and authentication-bound", () => {
  const peer = "abcdef123456";
  const repo = "11111111-1111-1111-1111-111111111111";
  const bound = `\u001b[32mSession bound to peer ${peer} and repo ${repo}\u001b[0m`;
  const completed = `P2P mesh connector handshake completed authenticated_peer_id="${peer}"`;

  for (const logs of [bound, completed]) {
    const result = meshEvidenceCount(logs, peer, repo);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout.trim(), "1");
  }

  for (const logs of [
    `Handling SyncHello from ${peer} for repo ${repo}`,
    `Session bound to peer deadbeef0000 and repo ${repo}`,
    `Session bound to peer ${peer} and repo 22222222-2222-2222-2222-222222222222`,
  ]) {
    const result = meshEvidenceCount(logs, peer, repo);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout.trim(), "0");
  }

  const emptyPeer = meshEvidenceCount(bound, "", repo);
  assert.notEqual(emptyPeer.status, 0, "empty expected peer identity must fail closed");
});

test("release workflow validates the complete tag set before the first push", () => {
  const validation = releaseWorkflow.indexOf("validate-release-image-tags.sh");
  const versionOutput = releaseWorkflow.indexOf("version_tag=%s");
  const latestOutput = releaseWorkflow.indexOf("latest_tag=%s");
  const versionPush = releaseWorkflow.indexOf('docker push "$VERSION_TAG"');
  const latestPush = releaseWorkflow.indexOf('docker push "$LATEST_TAG"');
  assert.ok(validation >= 0, "tag-set validation marker must exist");
  assert.ok(versionOutput > validation, "validated version tag must be exported");
  assert.ok(latestOutput > versionOutput, "validated latest tag must be exported after version");
  assert.ok(versionPush > validation, "version tag validation must precede its push");
  assert.ok(latestPush > validation, "latest tag validation must precede its push");
  assert.match(releaseWorkflow, /"\$\{validated\[0\]\}" "\$\{validated\[1\]:-\}"/);
});

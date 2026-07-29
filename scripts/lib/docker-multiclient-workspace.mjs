import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

function dockerRuntime() {
  const docker = process.env.DEVE_DOCKER_MULTI_DOCKER_BIN ?? "docker";
  const container = process.env.DEVE_DOCKER_MULTI_CONTAINER_ID;
  assert.ok(container, "DEVE_DOCKER_MULTI_CONTAINER_ID is required for product journeys");
  return { docker, container };
}

export function configureFirstRepoProjectionBase() {
  const { docker, container } = dockerRuntime();
  execFileSync(
    docker,
    [
      "exec",
      container,
      "deve",
      "config",
      "set",
      "repo_creation_projection_base",
      "/notes",
    ],
    { stdio: ["ignore", "ignore", "pipe"], timeout: 30000 },
  );
}

function locatorStringField(block, field) {
  const match = block.match(new RegExp(`^${field}\\s*=\\s*(['"])([^'"\\r\\n]+)\\1\\s*$`, "mu"));
  assert.ok(match, `Projection Locator record is missing ${field}`);
  return match[2];
}

export function selectWorkspaceRoot(locatorContent, repoId) {
  assert.match(repoId, /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u);
  const records = locatorContent.split(/^\[\[locators\]\]\s*$/mu).slice(1).map((block) => ({
    repoId: locatorStringField(block, "repo_id"),
    segment: locatorStringField(block, "workspace_segment"),
    base: locatorStringField(block, "projection_base_abs"),
  }));
  const matches = records.filter((record) => record.repoId === repoId);
  assert.equal(matches.length, 1, `expected one locator for repo ${repoId}`);
  const [{ base, segment }] = matches;
  assert.equal(base, "/notes", `Docker smoke projection base must be /notes, observed ${base}`);
  assert.match(segment, /^(?:[a-zA-Z0-9._-]+--)?[0-9a-f-]+$/u);
  return `${base}/${segment}`;
}

export function validateWorkspaceIdentity(identityContent, repoId) {
  assert.match(identityContent, /^version\s*=\s*1\s*$/mu);
  assert.equal(locatorStringField(identityContent, "repo_id"), repoId);
}

function dockerWorkspaceRoot(repoId) {
  const { docker, container } = dockerRuntime();
  const locatorContent = execFileSync(
    docker,
    ["exec", container, "cat", "/data/ledger/.host/projection-locators.toml"],
    { encoding: "utf8", timeout: 30000 },
  );
  return {
    docker,
    container,
    workspace: selectWorkspaceRoot(locatorContent, repoId),
  };
}

export function prepareRemovalPreservationFixture(repoId) {
  const { docker, container, workspace } = dockerWorkspaceRoot(repoId);
  execFileSync(
    docker,
    [
      "exec",
      container,
      "sh",
      "-c",
      'set -eu; root="$1"; printf "# preserved\\n" > "$root/preserved.md"; printf "unknown\\n" > "$root/unknown.bin"; mkdir -p "$root/.git"; printf "[core]\\n" > "$root/.git/config"',
      "_",
      workspace,
    ],
    { stdio: ["ignore", "ignore", "pipe"], timeout: 30000 },
  );
  return {
    workspace,
    expectedHash: readPreservationHash(docker, container, workspace),
  };
}

function readPreservationHash(docker, container, workspace) {
  return execFileSync(
    docker,
    [
      "exec",
      container,
      "sh",
      "-c",
      'set -eu; root="$1"; sha256sum "$root/preserved.md" "$root/unknown.bin" "$root/.git/config" "$root/.gitignore"',
      "_",
      workspace,
    ],
    { encoding: "utf8", timeout: 30000 },
  );
}

export function assertRemovalPreservation(repoId, preservation) {
  const { docker, container } = dockerRuntime();
  const { workspace, expectedHash } = preservation;
  execFileSync(
    docker,
    [
      "exec",
      container,
      "sh",
      "-c",
      'set -eu; root="$1"; repo_id="$2"; test -f "$root/preserved.md"; test -f "$root/unknown.bin"; test -f "$root/.git/config"; test -f "$root/.gitignore"; test ! -e "$root/.notegit"; test ! -e "/data/ledger/local/${repo_id}.redb"',
      "_",
      workspace,
      repoId,
    ],
    { stdio: ["ignore", "ignore", "pipe"], timeout: 30000 },
  );
  assert.equal(
    readPreservationHash(docker, container, workspace),
    expectedHash,
    "workspace, unknown, ignore, and Git bytes must remain unchanged",
  );
}

export function mutateWorkspaceFile(repoId, path, content) {
  const { docker, container, workspace } = dockerWorkspaceRoot(repoId);
  const identityContent = execFileSync(
    docker,
    [
      "exec",
      container,
      "sh",
      "-c",
      'test ! -L "$1" && test -f "$1" && cat "$1"',
      "_",
      `${workspace}/.notegit/identity.toml`,
    ],
    { encoding: "utf8", timeout: 30000 },
  );
  validateWorkspaceIdentity(identityContent, repoId);
  execFileSync(
    docker,
    ["exec", "-i", container, "tee", `${workspace}/${path}`],
    { input: content, stdio: ["pipe", "ignore", "pipe"], timeout: 30000 },
  );
}

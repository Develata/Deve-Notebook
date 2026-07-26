import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const registry = JSON.parse(readFileSync(
  new URL("../docs/registry/acceptance-producers.json", import.meta.url),
  "utf8",
));
const linuxWorkflow = readFileSync(
  new URL("../.github/workflows/release-candidate.yml", import.meta.url),
  "utf8",
);
const nativeWorkflow = readFileSync(
  new URL("../.github/workflows/release-native.yml", import.meta.url),
  "utf8",
);

function producer(id) {
  const value = registry.producers.find(({ producer_id: producerId }) => producerId === id);
  assert.ok(value, `missing producer ${id}`);
  return value;
}

function cargoFilters(value) {
  return value.steps
    .filter(({ program }) => program === "cargo")
    .map(({ args }) => args.map((arg) => arg.literal ?? `<env:${arg.env}>`).join(" "));
}

test("Linux and Windows repo lifecycle producers own unique evidence identities", () => {
  const linux = producer("repo-lifecycle.process-linux");
  const windows = producer("repo-lifecycle.process-windows");
  assert.deepEqual(linux.host_os, ["linux"]);
  assert.deepEqual(windows.host_os, ["windows"]);
  assert.deepEqual(linux.evidence_ids, [
    "smoke.repo-lifecycle.process-linux",
    "smoke.watcher.convergence-linux",
  ]);
  assert.deepEqual(windows.evidence_ids, [
    "smoke.repo-lifecycle.process-windows",
    "smoke.watcher.convergence-windows",
  ]);

  const expectedFilters = [
    "repo_lifecycle_runtime::tests",
    "repo_lifecycle_job_runtime::tests::removal",
    "commands::repo_remove::output::tests::lifecycle_outcomes_cross_process_boundary",
    "--test repo_removal_cli_test",
    "deve_core --lib watcher_",
    "--test watcher_platform_fs",
    "deve_cli --lib watcher_",
    "deve_cli --lib standalone_watch",
  ];
  for (const current of [linux, windows]) {
    const filters = cargoFilters(current);
    for (const expected of expectedFilters) {
      assert.ok(filters.some((command) => command.includes(expected)), `${current.producer_id}: ${expected}`);
    }
    assert.ok(current.steps.some(({ program, args }) =>
      program === "node"
      && args.some(({ literal }) => literal === "scripts/repo-lifecycle-producer-contract.test.mjs")));
  }

  const allEvidence = registry.producers.flatMap(({ evidence_ids: ids }) => ids);
  assert.equal(new Set(allEvidence).size, allEvidence.length);
});

test("candidate workflows bind each process producer to the matching host receipt root", () => {
  assert.match(linuxWorkflow, /--producer repo-lifecycle\.process-linux/);
  assert.match(linuxWorkflow, /deve-acceptance-repo-lifecycle-linux/);
  assert.match(nativeWorkflow, /--producer repo-lifecycle\.process-windows/);
  assert.match(nativeWorkflow, /deve-acceptance-desktop-repo-lifecycle/);
  assert.match(nativeWorkflow, /path: \$\{\{ runner\.temp \}\}\/deve-acceptance-desktop-\*/);
});

test("browser producers own UI journeys while process tests remain separate", () => {
  const docker = producer("docker.multiclient-product");
  assert.deepEqual(docker.steps.map(({ program }) => program), ["bash"]);
  assert.equal(
    docker.steps[0].args[0].literal,
    "scripts/smoke-docker-multiclient.sh",
  );

  assert.deepEqual(producer("desktop.local-backend").claims_env, {
    "smoke.desktop.local-backend": "DEVE_DESKTOP_LOCAL_AUTHORITY_EVIDENCE_PATH",
  });
  assert.deepEqual(producer("desktop.remote-browser").claims_env, {
    "smoke.desktop.remote-browser": "DEVE_DESKTOP_REMOTE_AUTHORITY_EVIDENCE_PATH",
  });
  for (const id of ["android.local-backend", "android.remote-browser"]) {
    assert.ok(producer(id).artifacts.includes("scripts/lib/android-writable-evidence.mjs"));
  }
});

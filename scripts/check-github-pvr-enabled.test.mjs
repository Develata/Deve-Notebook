import assert from "node:assert/strict";
import {
  closeSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  API_VERSION,
  CLAIMS_ENV,
  GH_TIMEOUT_MILLISECONDS,
  GITHUB_HOST,
  PVR_ENDPOINT,
  REPOSITORY,
  REPOSITORY_ENDPOINT,
  ghJson,
  githubArgs,
  probePvr,
  validatePvr,
  validateRepository,
  writeClaimsAtomically,
} from "./check-github-pvr-enabled.mjs";

test("GitHub arguments are read-only and fixed to the exact repository", () => {
  for (const endpoint of [REPOSITORY_ENDPOINT, PVR_ENDPOINT]) {
    const args = githubArgs(endpoint);
    assert.deepEqual(args.slice(0, 5), [
      "api",
      "--hostname",
      GITHUB_HOST,
      "--method",
      "GET",
    ]);
    assert.ok(args.includes(`X-GitHub-Api-Version: ${API_VERSION}`));
    assert.equal(args.at(-1), endpoint);
    assert.equal(args.includes("PUT"), false);
    assert.equal(args.includes("DELETE"), false);
  }
  assert.throws(() => githubArgs("repos/other/repository"));
});

test("repository identity admits only the exact public repository", () => {
  assert.doesNotThrow(() =>
    validateRepository({
      full_name: REPOSITORY,
      private: false,
      visibility: "public",
    }),
  );
  for (const invalid of [
    { full_name: "Other/Repo", private: false, visibility: "public" },
    { full_name: REPOSITORY, private: true, visibility: "private" },
    { full_name: REPOSITORY, private: false, visibility: "internal" },
  ]) {
    assert.throws(() => validateRepository(invalid));
  }
});

test("PVR response admits only the exact enabled object", () => {
  assert.doesNotThrow(() => validatePvr({ enabled: true }));
  for (const invalid of [
    { enabled: false },
    {},
    { enabled: true, source: "cached" },
    null,
    [],
  ]) {
    assert.throws(() => validatePvr(invalid));
  }
});

test("probe reads repository identity before PVR state", () => {
  const endpoints = [];
  const claims = probePvr((endpoint) => {
    endpoints.push(endpoint);
    return endpoint === REPOSITORY_ENDPOINT
      ? { full_name: REPOSITORY, private: false, visibility: "public" }
      : { enabled: true };
  });
  assert.deepEqual(endpoints, [REPOSITORY_ENDPOINT, PVR_ENDPOINT]);
  assert.deepEqual(claims, { enabled: true });
});

test("gh JSON failures remain fail-closed", () => {
  assert.throws(() =>
    ghJson(PVR_ENDPOINT, () => ({
      status: 1,
      stdout: "",
      stderr: "denied",
    })),
  );
  assert.throws(() =>
    ghJson(PVR_ENDPOINT, () => ({
      status: 0,
      stdout: "not-json",
      stderr: "",
    })),
  );
});

test("gh invocation has an internal timeout and never forwards stderr", () => {
  let observedOptions;
  assert.throws(
    () =>
      ghJson(PVR_ENDPOINT, (_program, _args, options) => {
        observedOptions = options;
        return {
          status: 1,
          stdout: "",
          stderr: "secret-bearing external diagnostic",
        };
      }),
    (error) => {
      assert.equal(error.message.includes("secret-bearing"), false);
      return true;
    },
  );
  assert.equal(observedOptions.timeout, GH_TIMEOUT_MILLISECONDS);
});

test("claims use an atomic same-directory temporary file", () => {
  const directory = mkdtempSync(join(tmpdir(), "deve-pvr-"));
  const outputPath = join(directory, "claims.json");
  try {
    writeClaimsAtomically(outputPath, { enabled: true });
    assert.deepEqual(JSON.parse(readFileSync(outputPath, "utf8")), {
      enabled: true,
    });
    assert.deepEqual(readdirSync(directory), ["claims.json"]);
    assert.throws(() =>
      writeClaimsAtomically(join("relative", "claims.json"), { enabled: true }),
    );
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("claims cleanup survives rename and close failures", () => {
  for (const failure of ["rename", "close"]) {
    const directory = mkdtempSync(join(tmpdir(), `deve-pvr-${failure}-`));
    const outputPath = join(directory, "claims.json");
    let closeCalls = 0;
    try {
      const overrides =
        failure === "rename"
          ? {
              renameSync() {
                throw new Error("injected rename failure");
              },
            }
          : {
              closeSync(descriptor) {
                closeCalls += 1;
                if (closeCalls === 1) {
                  throw new Error("injected close failure");
                }
                closeSync(descriptor);
              },
            };
      assert.throws(() =>
        writeClaimsAtomically(outputPath, { enabled: true }, overrides),
      );
      assert.deepEqual(readdirSync(directory), []);
      if (failure === "close") {
        assert.equal(closeCalls, 2);
      }
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  }
});

test("claims cleanup never deletes a temporary file it did not create", () => {
  const directory = mkdtempSync(join(tmpdir(), "deve-pvr-owned-"));
  const outputPath = join(directory, "claims.json");
  const temporaryPath = `${outputPath}.tmp-${process.pid}-occupied`;
  try {
    writeFileSync(temporaryPath, "other writer", "utf8");
    assert.throws(() =>
      writeClaimsAtomically(
        outputPath,
        { enabled: true },
        { randomUUID: () => "occupied" },
      ),
    );
    assert.equal(readFileSync(temporaryPath, "utf8"), "other writer");
    assert.deepEqual(readdirSync(directory), [
      "claims.json.tmp-" + process.pid + "-occupied",
    ]);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("producer registry binds the PVR evidence to the read-only script", () => {
  const registry = JSON.parse(
    readFileSync(
      new URL("../docs/registry/acceptance-producers.json", import.meta.url),
      "utf8",
    ),
  );
  const producer = registry.producers.find(
    (candidate) => candidate.producer_id === "github.pvr-enabled",
  );
  assert.ok(producer);
  assert.deepEqual(producer.evidence_ids, ["github.pvr.enabled"]);
  assert.deepEqual(producer.required_env, []);
  assert.deepEqual(producer.environment, {});
  assert.deepEqual(producer.claims_env, {
    "github.pvr.enabled": CLAIMS_ENV,
  });
  assert.deepEqual(producer.artifacts, [
    "scripts/check-github-pvr-enabled.mjs",
  ]);
  assert.deepEqual(producer.steps, [
    {
      program: "node",
      args: [{ literal: "scripts/check-github-pvr-enabled.mjs" }],
    },
  ]);
});

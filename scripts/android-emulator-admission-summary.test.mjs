import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  ADMISSION_VARIANTS,
  summarizeAdmissionResults,
} from "./android-emulator-admission-summary.mjs";

const HEAD = "0123456789abcdef0123456789abcdef01234567";
const APK_SHA256 = "a".repeat(64);

function cycle(cycleNumber, outcome = "passed") {
  return {
    cycle: cycleNumber,
    outcome,
    phase: outcome === "passed" ? "complete" : "install",
    exitStatus: outcome === "passed" ? 0 : 1,
    cleanupStatus: 0,
    failureClass: outcome === "passed" ? null : "binder_epipe",
    systemServerPidBefore: "123",
    systemServerPidAfter: outcome === "passed" ? "123" : null,
  };
}

function result(variantId, { stable = false, headSha = HEAD, cycles = 3 } = {}) {
  const variant = ADMISSION_VARIANTS.find((candidate) => candidate.id === variantId);
  assert.ok(variant);
  return {
    schemaVersion: 1,
    kind: "android-emulator-admission-diagnostic",
    complete: true,
    headSha,
    variantId,
    emulatorSource: variant.emulatorSource,
    emulatorVersion: variant.emulatorSource === "pinned" ? "36.2.12.0" : "36.3.10.0",
    emulatorBuildId: variant.emulatorSource === "pinned" ? "14394846" : "15000000",
    emulatorProbeStatus: "0",
    sdkEmulatorRevision: "36.3.10",
    apiLevel: variant.apiLevel,
    systemTarget: "google_apis",
    systemImageRevision: variant.apiLevel === "37.0" ? "2" : "1",
    architecture: "x86_64",
    apkSha256: APK_SHA256,
    requestedCycles: cycles,
    stable,
    harnessError: null,
    cycles: Array.from(
      { length: cycles },
      (_, index) => cycle(index + 1, stable ? "passed" : index === 0 ? "failed" : "passed"),
    ),
  };
}

function withResults(results, callback) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "deve-android-admission-summary-"));
  try {
    for (const entry of results) {
      fs.writeFileSync(
        path.join(root, `${entry.variantId}.json`),
        `${JSON.stringify(entry, null, 2)}\n`,
      );
    }
    return callback(root);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
}

test("recommends the least-divergent complete stable variant", () => {
  withResults([
    result("current-pinned-api37"),
    result("sdk-api37", { stable: true }),
    result("pinned-api36-1", { stable: true }),
  ], (rootDir) => {
    const summary = summarizeAdmissionResults({
      rootDir,
      expectedHead: HEAD,
      expectedCycles: 3,
    });
    assert.deepEqual(summary.stableVariantIds, ["sdk-api37", "pinned-api36-1"]);
    assert.equal(summary.recommendedVariantId, "sdk-api37");
    assert.match(summary.markdown, /\| sdk-api37 \| sdk \| 37\.0 \| 3\/3 \| stable \|/);
  });
});

test("prefers the unchanged control when it is stable", () => {
  withResults([
    result("current-pinned-api37", { stable: true }),
    result("sdk-api37", { stable: true }),
    result("pinned-api36-1"),
  ], (rootDir) => {
    const summary = summarizeAdmissionResults({
      rootDir,
      expectedHead: HEAD,
      expectedCycles: 3,
    });
    assert.equal(summary.recommendedVariantId, "current-pinned-api37");
  });
});

test("rejects missing, duplicate, or identity-drifted matrix results", () => {
  withResults([
    result("current-pinned-api37"),
    result("sdk-api37"),
  ], (rootDir) => {
    assert.throws(
      () => summarizeAdmissionResults({
        rootDir,
        expectedHead: HEAD,
        expectedCycles: 3,
      }),
      /expected exactly 3 result files/,
    );
  });

  withResults([
    result("current-pinned-api37"),
    result("sdk-api37", { headSha: "f".repeat(40) }),
    result("pinned-api36-1"),
  ], (rootDir) => {
    assert.throws(
      () => summarizeAdmissionResults({
        rootDir,
        expectedHead: HEAD,
        expectedCycles: 3,
      }),
      /headSha mismatch/,
    );
  });
});

test("rejects incomplete cycles, false stable claims, and harness errors", () => {
  const incomplete = result("sdk-api37");
  incomplete.cycles.pop();
  const falseStable = result("pinned-api36-1");
  falseStable.stable = true;
  const harnessError = result("current-pinned-api37");
  harnessError.complete = false;
  harnessError.harnessError = "sdk install failed";

  for (const replacement of [incomplete, falseStable, harnessError]) {
    const entries = ADMISSION_VARIANTS.map(({ id }) => (
      id === replacement.variantId ? replacement : result(id)
    ));
    withResults(entries, (rootDir) => {
      assert.throws(
        () => summarizeAdmissionResults({
          rootDir,
          expectedHead: HEAD,
          expectedCycles: 3,
        }),
      );
    });
  }
});

test("requires exactly three cold-boot cycles", () => {
  withResults(ADMISSION_VARIANTS.map(({ id }) => result(id)), (rootDir) => {
    assert.throws(
      () => summarizeAdmissionResults({
        rootDir,
        expectedHead: HEAD,
        expectedCycles: 2,
      }),
      /exactly 3/,
    );
  });
});

test("rejects missing or drifted experiment identities", () => {
  const cases = [
    {
      variantId: "sdk-api37",
      mutate(entry) {
        delete entry.apkSha256;
      },
      pattern: /apkSha256/,
    },
    {
      variantId: "sdk-api37",
      mutate(entry) {
        entry.apkSha256 = "b".repeat(64);
      },
      pattern: /APK identity drifted/,
    },
    {
      variantId: "pinned-api36-1",
      mutate(entry) {
        entry.emulatorBuildId = "99999999";
      },
      pattern: /pinned emulator identity drifted/,
    },
    {
      variantId: "sdk-api37",
      mutate(entry) {
        entry.systemImageRevision = "99";
      },
      pattern: /API 37 system-image identity drifted/,
    },
  ];
  for (const { variantId, mutate, pattern } of cases) {
    const entries = ADMISSION_VARIANTS.map(({ id }) => result(id));
    mutate(entries.find((entry) => entry.variantId === variantId));
    withResults(entries, (rootDir) => {
      assert.throws(
        () => summarizeAdmissionResults({
          rootDir,
          expectedHead: HEAD,
          expectedCycles: 3,
        }),
        pattern,
      );
    });
  }
});

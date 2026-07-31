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

function cycle(cycleNumber, outcome = "passed", rendererPair = "swiftshader swangle") {
  return {
    cycle: cycleNumber,
    outcome,
    phase: outcome === "passed" ? "complete" : "install",
    exitStatus: outcome === "passed" ? 0 : 1,
    cleanupStatus: 0,
    failureClass: outcome === "passed" ? null : "binder_epipe",
    systemServerPidBefore: "123",
    systemServerPidAfter: outcome === "passed" ? "123" : null,
    rendererPair,
  };
}

function result(variantId, { stable = false, headSha = HEAD, cycles = 3 } = {}) {
  const variant = ADMISSION_VARIANTS.find((candidate) => candidate.id === variantId);
  assert.ok(variant);
  const rendererPair = variant.gpuMode === "swangle"
    ? "swiftshader swangle"
    : "swiftshader swiftshader";
  return {
    schemaVersion: 1,
    kind: "android-emulator-admission-diagnostic",
    complete: true,
    headSha,
    variantId,
    emulatorSource: variant.emulatorSource,
    gpuMode: variant.gpuMode,
    emulatorVersion: "36.2.12.0",
    emulatorBuildId: "14394846",
    emulatorProbeStatus: "0",
    sdkEmulatorRevision: "36.3.10",
    apiLevel: variant.apiLevel,
    systemTarget: "google_apis",
    systemImageRevision: "2",
    architecture: "x86_64",
    apkSha256: APK_SHA256,
    requestedCycles: cycles,
    stable,
    harnessError: null,
    cycles: Array.from(
      { length: cycles },
      (_, index) => cycle(
        index + 1,
        stable ? "passed" : index === 0 ? "failed" : "passed",
        rendererPair,
      ),
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
    result("pinned-api37-swangle"),
    result("pinned-api37-software", { stable: true }),
    result("pinned-api37-swiftshader", { stable: true }),
  ], (rootDir) => {
    const summary = summarizeAdmissionResults({
      rootDir,
      expectedHead: HEAD,
      expectedCycles: 3,
    });
    assert.deepEqual(
      summary.stableVariantIds,
      ["pinned-api37-software", "pinned-api37-swiftshader"],
    );
    assert.equal(summary.recommendedVariantId, "pinned-api37-software");
    assert.match(
      summary.markdown,
      /\| pinned-api37-software \| software \| swiftshader swiftshader \| 3\/3 \| stable \|/,
    );
  });
});

test("prefers the unchanged control when it is stable", () => {
  withResults([
    result("pinned-api37-swangle", { stable: true }),
    result("pinned-api37-software", { stable: true }),
    result("pinned-api37-swiftshader"),
  ], (rootDir) => {
    const summary = summarizeAdmissionResults({
      rootDir,
      expectedHead: HEAD,
      expectedCycles: 3,
    });
    assert.equal(summary.recommendedVariantId, "pinned-api37-swangle");
  });
});

test("does not recommend a stable alias of the unstable control renderer", () => {
  const entries = [
    result("pinned-api37-swangle"),
    result("pinned-api37-software", { stable: true }),
    result("pinned-api37-swiftshader", { stable: true }),
  ];
  for (const entry of entries.slice(1)) {
    for (const cycleResult of entry.cycles) {
      cycleResult.rendererPair = "swiftshader swangle";
    }
  }
  withResults(entries, (rootDir) => {
    const summary = summarizeAdmissionResults({
      rootDir,
      expectedHead: HEAD,
      expectedCycles: 3,
    });
    assert.equal(summary.recommendedVariantId, null);
  });
});

test("rejects missing, duplicate, or identity-drifted matrix results", () => {
  withResults([
    result("pinned-api37-swangle"),
    result("pinned-api37-software"),
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
    result("pinned-api37-swangle"),
    result("pinned-api37-software", { headSha: "f".repeat(40) }),
    result("pinned-api37-swiftshader"),
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
  const incomplete = result("pinned-api37-software");
  incomplete.cycles.pop();
  const falseStable = result("pinned-api37-swiftshader");
  falseStable.stable = true;
  const harnessError = result("pinned-api37-swangle");
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
      variantId: "pinned-api37-software",
      mutate(entry) {
        delete entry.apkSha256;
      },
      pattern: /apkSha256/,
    },
    {
      variantId: "pinned-api37-software",
      mutate(entry) {
        entry.apkSha256 = "b".repeat(64);
      },
      pattern: /APK identity drifted/,
    },
    {
      variantId: "pinned-api37-swiftshader",
      mutate(entry) {
        entry.emulatorBuildId = "99999999";
      },
      pattern: /pinned emulator identity drifted/,
    },
    {
      variantId: "pinned-api37-software",
      mutate(entry) {
        entry.systemImageRevision = "99";
      },
      pattern: /API 37 system-image identity drifted across renderer variants/,
    },
    {
      variantId: "pinned-api37-swiftshader",
      mutate(entry) {
        entry.gpuMode = "software";
      },
      pattern: /gpuMode mismatch/,
    },
    {
      variantId: "pinned-api37-software",
      mutate(entry) {
        entry.cycles[1].rendererPair = "swiftshader swangle";
      },
      pattern: /renderer identity drifted across cycles/,
    },
    {
      variantId: "pinned-api37-software",
      mutate(entry) {
        delete entry.cycles[1].rendererPair;
      },
      pattern: /rendererPair is invalid/,
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

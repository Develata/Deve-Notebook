import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

export const ADMISSION_VARIANTS = Object.freeze([
  Object.freeze({
    id: "pinned-api37-swangle",
    emulatorSource: "pinned",
    apiLevel: "37.0",
    gpuMode: "swangle",
  }),
  Object.freeze({
    id: "pinned-api37-software",
    emulatorSource: "pinned",
    apiLevel: "37.0",
    gpuMode: "software",
  }),
  Object.freeze({
    id: "pinned-api37-swiftshader",
    emulatorSource: "pinned",
    apiLevel: "37.0",
    gpuMode: "swiftshader",
  }),
]);

function requireValue(condition, message) {
  if (!condition) throw new Error(`android-emulator-admission-summary: ${message}`);
}

function jsonFiles(rootDir) {
  const files = [];
  const visit = (directory) => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const candidate = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        visit(candidate);
      } else if (entry.isFile() && entry.name.endsWith(".json")) {
        files.push(candidate);
      } else if (entry.isSymbolicLink()) {
        throw new Error(`android-emulator-admission-summary: symlink is forbidden: ${candidate}`);
      }
    }
  };
  visit(rootDir);
  return files.sort();
}

function validateCycle(entry, expectedCycle) {
  requireValue(entry && typeof entry === "object", `cycle ${expectedCycle} must be an object`);
  requireValue(entry.cycle === expectedCycle, `expected cycle ${expectedCycle}, found ${entry.cycle}`);
  requireValue(["passed", "failed"].includes(entry.outcome), `cycle ${expectedCycle} has invalid outcome`);
  requireValue(Number.isInteger(entry.exitStatus), `cycle ${expectedCycle} exitStatus must be an integer`);
  requireValue(
    Number.isInteger(entry.cleanupStatus) && entry.cleanupStatus >= 0,
    `cycle ${expectedCycle} cleanupStatus must be a non-negative integer`,
  );
  requireValue(typeof entry.phase === "string" && entry.phase.length > 0, `cycle ${expectedCycle} phase is missing`);
  requireValue(
    /^[a-z0-9_-]+ [a-z0-9_-]+$/.test(entry.rendererPair),
    `cycle ${expectedCycle} rendererPair is invalid`,
  );
  if (entry.outcome === "passed") {
    requireValue(entry.exitStatus === 0, `passed cycle ${expectedCycle} must have exitStatus 0`);
    requireValue(entry.phase === "complete", `passed cycle ${expectedCycle} must finish at complete`);
    requireValue(entry.failureClass === null, `passed cycle ${expectedCycle} cannot have a failure class`);
    requireValue(
      /^[1-9][0-9]*$/.test(entry.systemServerPidBefore)
        && entry.systemServerPidBefore === entry.systemServerPidAfter,
      `passed cycle ${expectedCycle} must preserve system_server PID`,
    );
  } else {
    requireValue(entry.exitStatus !== 0, `failed cycle ${expectedCycle} cannot have exitStatus 0`);
    requireValue(
      typeof entry.failureClass === "string" && entry.failureClass.length > 0,
      `failed cycle ${expectedCycle} must have a failure class`,
    );
  }
}

function validateResult(result, variant, expectedHead, expectedCycles, file) {
  const scope = path.basename(file);
  requireValue(result && typeof result === "object", `${scope} must contain a JSON object`);
  requireValue(result.schemaVersion === 1, `${scope} schemaVersion must be 1`);
  requireValue(
    result.kind === "android-emulator-admission-diagnostic",
    `${scope} has unexpected kind`,
  );
  requireValue(result.complete === true, `${scope} is incomplete`);
  requireValue(result.harnessError === null, `${scope} contains harness error: ${result.harnessError}`);
  requireValue(result.headSha === expectedHead, `${scope} headSha mismatch`);
  requireValue(result.variantId === variant.id, `${scope} variantId mismatch`);
  requireValue(
    result.emulatorSource === variant.emulatorSource,
    `${scope} emulatorSource mismatch`,
  );
  requireValue(result.apiLevel === variant.apiLevel, `${scope} apiLevel mismatch`);
  requireValue(result.gpuMode === variant.gpuMode, `${scope} gpuMode mismatch`);
  requireValue(result.systemTarget === "google_apis", `${scope} systemTarget mismatch`);
  requireValue(result.architecture === "x86_64", `${scope} architecture mismatch`);
  requireValue(
    /^[0-9a-f]{64}$/.test(result.apkSha256),
    `${scope} apkSha256 must be a lowercase SHA-256`,
  );
  requireValue(
    /^[0-9]+([.][0-9]+){3}$/.test(result.emulatorVersion),
    `${scope} emulatorVersion is invalid`,
  );
  requireValue(/^[1-9][0-9]*$/.test(result.emulatorBuildId), `${scope} emulatorBuildId is invalid`);
  requireValue(/^[0-9]+$/.test(result.emulatorProbeStatus), `${scope} emulatorProbeStatus is invalid`);
  requireValue(
    /^[0-9]+([.][0-9]+)*$/.test(result.sdkEmulatorRevision),
    `${scope} sdkEmulatorRevision is invalid`,
  );
  requireValue(
    /^[0-9]+([.][0-9]+)*$/.test(result.systemImageRevision),
    `${scope} systemImageRevision is invalid`,
  );
  requireValue(
    result.requestedCycles === expectedCycles,
    `${scope} requestedCycles mismatch`,
  );
  requireValue(Array.isArray(result.cycles), `${scope} cycles must be an array`);
  requireValue(
    result.cycles.length === expectedCycles,
    `${scope} expected ${expectedCycles} cycles, found ${result.cycles.length}`,
  );
  result.cycles.forEach((entry, index) => validateCycle(entry, index + 1));
  const rendererPairs = new Set(result.cycles.map((entry) => entry.rendererPair));
  requireValue(
    rendererPairs.size === 1,
    `${scope} renderer identity drifted across cycles`,
  );
  const computedStable = result.cycles.every((entry) => entry.outcome === "passed");
  requireValue(result.stable === computedStable, `${scope} stable claim disagrees with cycles`);
  return { ...result, observedRendererPair: [...rendererPairs][0] };
}

function renderMarkdown(results, stableVariantIds, recommendedVariantId) {
  const lines = [
    "## Android emulator admission diagnostic",
    "",
    "| Variant | Requested GPU | Observed Vulkan/GLES | Passed cycles | Result |",
    "|---|---|---|---:|---|",
  ];
  for (const result of results) {
    const passed = result.cycles.filter((entry) => entry.outcome === "passed").length;
    lines.push(
      `| ${result.variantId} | ${result.gpuMode} | ${result.observedRendererPair} | `
      + `${passed}/${result.requestedCycles} | ${result.stable ? "stable" : "unstable"} |`,
    );
  }
  lines.push(
    "",
    `Stable variants: ${stableVariantIds.length > 0 ? stableVariantIds.join(", ") : "none"}`,
    `Recommended least-divergent variant: ${recommendedVariantId ?? "none"}`,
    "",
    "> Diagnostic only: these artifacts are not acceptance receipts and do not change the release gate.",
    "",
  );
  return lines.join("\n");
}

export function summarizeAdmissionResults({ rootDir, expectedHead, expectedCycles }) {
  requireValue(fs.statSync(rootDir).isDirectory(), "results directory is not a directory");
  requireValue(/^[0-9a-f]{40}$/.test(expectedHead), "expected HEAD must be a lowercase SHA-1");
  requireValue(
    expectedCycles === 3,
    "expected cycles must be exactly 3",
  );

  const files = jsonFiles(rootDir);
  requireValue(
    files.length === ADMISSION_VARIANTS.length,
    `expected exactly ${ADMISSION_VARIANTS.length} result files, found ${files.length}`,
  );
  const byVariant = new Map();
  for (const file of files) {
    const parsed = JSON.parse(fs.readFileSync(file, "utf8"));
    requireValue(
      typeof parsed.variantId === "string" && !byVariant.has(parsed.variantId),
      `duplicate or missing variantId in ${path.basename(file)}`,
    );
    byVariant.set(parsed.variantId, { parsed, file });
  }

  const results = ADMISSION_VARIANTS.map((variant) => {
    const entry = byVariant.get(variant.id);
    requireValue(Boolean(entry), `missing matrix result ${variant.id}`);
    return validateResult(entry.parsed, variant, expectedHead, expectedCycles, entry.file);
  });
  requireValue(
    byVariant.size === ADMISSION_VARIANTS.length,
    "matrix contains an unrecognized variant",
  );
  requireValue(
    new Set(results.map((result) => result.apkSha256)).size === 1,
    "matrix APK identity drifted across variants",
  );
  requireValue(
    new Set(results.map((result) => (
      `${result.emulatorVersion}/${result.emulatorBuildId}/${result.emulatorProbeStatus}`
    ))).size === 1,
    "pinned emulator identity drifted across renderer variants",
  );
  requireValue(
    new Set(results.map((result) => result.systemImageRevision)).size === 1,
    "API 37 system-image identity drifted across renderer variants",
  );
  const stableVariantIds = results
    .filter((result) => result.stable)
    .map((result) => result.variantId);
  const control = results[0];
  const recommendedVariantId = control.stable
    ? control.variantId
    : results.find((result) => (
      result.stable && result.observedRendererPair !== control.observedRendererPair
    ))?.variantId ?? null;
  return {
    results,
    stableVariantIds,
    recommendedVariantId,
    markdown: renderMarkdown(results, stableVariantIds, recommendedVariantId),
  };
}

function parseArgs(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    requireValue(key?.startsWith("--") && value !== undefined, `invalid argument near ${key}`);
    requireValue(!values.has(key), `duplicate argument ${key}`);
    values.set(key, value);
  }
  for (const required of ["--results-dir", "--expected-head", "--expected-cycles"]) {
    requireValue(values.has(required), `missing ${required}`);
  }
  requireValue(values.size === 3, "unexpected CLI argument");
  return {
    rootDir: values.get("--results-dir"),
    expectedHead: values.get("--expected-head"),
    expectedCycles: Number.parseInt(values.get("--expected-cycles"), 10),
  };
}

function appendSummary(markdown) {
  if (process.env.GITHUB_STEP_SUMMARY) {
    fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, markdown);
  } else {
    process.stdout.write(markdown);
  }
}

function appendOutput(name, value) {
  if (process.env.GITHUB_OUTPUT) {
    fs.appendFileSync(process.env.GITHUB_OUTPUT, `${name}=${value}\n`);
  }
}

function main() {
  let summary;
  try {
    summary = summarizeAdmissionResults(parseArgs(process.argv.slice(2)));
    appendSummary(summary.markdown);
    appendOutput("stable_variants", summary.stableVariantIds.join(","));
    appendOutput("recommended_variant", summary.recommendedVariantId ?? "");
    requireValue(summary.recommendedVariantId !== null, "no variant passed every cold-boot cycle");
  } catch (error) {
    appendSummary(`## Android emulator admission diagnostic\n\nFailed closed: ${error.message}\n`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}

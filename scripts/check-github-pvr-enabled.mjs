import {
  closeSync,
  mkdirSync,
  openSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { dirname, isAbsolute, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const GITHUB_HOST = "github.com";
export const REPOSITORY = "Develata/Deve-Notebook";
export const API_VERSION = "2026-03-10";
export const REPOSITORY_ENDPOINT = `repos/${REPOSITORY}`;
export const PVR_ENDPOINT = `${REPOSITORY_ENDPOINT}/private-vulnerability-reporting`;
export const CLAIMS_ENV = "DEVE_GITHUB_PVR_CLAIMS";
export const GH_TIMEOUT_MILLISECONDS = 45_000;

function requireObject(value, label) {
  if (value === null || Array.isArray(value) || typeof value !== "object") {
    throw new Error(`${label} response must be a JSON object`);
  }
  return value;
}

export function githubArgs(endpoint) {
  if (endpoint !== REPOSITORY_ENDPOINT && endpoint !== PVR_ENDPOINT) {
    throw new Error(`unsupported GitHub API endpoint: ${endpoint}`);
  }
  return [
    "api",
    "--hostname",
    GITHUB_HOST,
    "--method",
    "GET",
    "-H",
    "Accept: application/vnd.github+json",
    "-H",
    `X-GitHub-Api-Version: ${API_VERSION}`,
    endpoint,
  ];
}

export function ghJson(endpoint, spawn = spawnSync) {
  const result = spawn("gh", githubArgs(endpoint), {
    encoding: "utf8",
    maxBuffer: 256 * 1024,
    shell: false,
    timeout: GH_TIMEOUT_MILLISECONDS,
    windowsHide: true,
  });
  if (result.error) {
    const category = result.error.code ?? "unknown";
    throw new Error(`GitHub API GET failed to start or timed out (${category})`);
  }
  if (result.status !== 0) {
    throw new Error(
      `GitHub API GET failed for ${endpoint} with exit ${result.status}`,
    );
  }
  try {
    return requireObject(JSON.parse(result.stdout), endpoint);
  } catch (error) {
    if (error instanceof SyntaxError) {
      throw new Error(`${endpoint} returned invalid JSON`);
    }
    throw error;
  }
}

export function validateRepository(value) {
  const repository = requireObject(value, "repository");
  if (
    repository.full_name !== REPOSITORY ||
    repository.private !== false ||
    repository.visibility !== "public"
  ) {
    throw new Error(
      `repository identity must be the public ${REPOSITORY} repository`,
    );
  }
}

export function validatePvr(value) {
  const pvr = requireObject(value, "private vulnerability reporting");
  const keys = Object.keys(pvr);
  if (keys.length !== 1 || keys[0] !== "enabled" || pvr.enabled !== true) {
    throw new Error(
      "private vulnerability reporting must return exactly {\"enabled\":true}",
    );
  }
}

export function probePvr(readJson = ghJson) {
  validateRepository(readJson(REPOSITORY_ENDPOINT));
  const pvr = readJson(PVR_ENDPOINT);
  validatePvr(pvr);
  return { enabled: true };
}

export function writeClaimsAtomically(outputPath, claims, overrides = {}) {
  if (
    typeof outputPath !== "string" ||
    outputPath.length === 0 ||
    !isAbsolute(outputPath)
  ) {
    throw new Error(`${CLAIMS_ENV} must name an absolute output path`);
  }

  const outputDirectory = dirname(outputPath);
  const fileSystem = {
    closeSync,
    mkdirSync,
    openSync,
    randomUUID,
    renameSync,
    unlinkSync,
    writeFileSync,
    ...overrides,
  };
  const temporaryPath = `${outputPath}.tmp-${process.pid}-${fileSystem.randomUUID()}`;
  fileSystem.mkdirSync(outputDirectory, { recursive: true });

  let descriptor;
  let primaryError;
  let temporaryOwned = false;
  try {
    descriptor = fileSystem.openSync(temporaryPath, "wx", 0o600);
    temporaryOwned = true;
    fileSystem.writeFileSync(
      descriptor,
      `${JSON.stringify(claims, null, 2)}\n`,
      "utf8",
    );
    fileSystem.closeSync(descriptor);
    descriptor = undefined;
    fileSystem.renameSync(temporaryPath, outputPath);
    return;
  } catch (error) {
    primaryError = error;
  }

  const cleanupErrors = [];
  if (descriptor !== undefined) {
    try {
      fileSystem.closeSync(descriptor);
    } catch (error) {
      cleanupErrors.push(error);
    }
  }
  if (temporaryOwned) {
    try {
      fileSystem.unlinkSync(temporaryPath);
    } catch (error) {
      if (error?.code !== "ENOENT") {
        cleanupErrors.push(error);
      }
    }
  }
  if (cleanupErrors.length > 0) {
    throw new AggregateError(
      [primaryError, ...cleanupErrors],
      "PVR claims write and temporary-file cleanup failed",
    );
  }
  throw primaryError;
}

export function main(environment = process.env) {
  const claims = probePvr();
  writeClaimsAtomically(environment[CLAIMS_ENV], claims);
  console.log(
    `github-pvr-enabled: ${REPOSITORY} returned exactly {"enabled":true}`,
  );
}

const invokedDirectly =
  process.argv[1] !== undefined &&
  pathToFileURL(resolve(process.argv[1])).href === import.meta.url;
if (invokedDirectly) {
  try {
    main();
  } catch (error) {
    console.error(`github-pvr-enabled: ${error.message}`);
    process.exitCode = 1;
  }
}

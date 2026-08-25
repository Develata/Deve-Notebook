#!/usr/bin/env node

import { createHash } from "node:crypto";
import { lstat, mkdir, readFile, readdir, realpath, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const MAX_FILES = 4096;
const MAX_FILE_BYTES = 32 * 1024 * 1024;
const MAX_TOTAL_BYTES = 128 * 1024 * 1024;
const HEAD_PATTERN = /^[0-9a-f]{40}$/;
const SHA256_PATTERN = /^[0-9a-f]{64}$/;

function compareCanonicalPath(left, right) {
  return Buffer.compare(Buffer.from(left, "utf8"), Buffer.from(right, "utf8"));
}

function fail(message) {
  throw new Error(`web-dist-artifact: ${message}`);
}

function parseArgs(argv) {
  const [command, ...rest] = argv;
  if (command !== "seal" && command !== "verify") {
    fail("expected seal or verify");
  }
  const values = new Map();
  for (let index = 0; index < rest.length; index += 2) {
    const key = rest[index];
    const value = rest[index + 1];
    if (!key?.startsWith("--") || value === undefined || values.has(key)) {
      fail(`invalid or repeated option ${key ?? "<missing>"}`);
    }
    values.set(key, value);
  }
  const allowed = new Set(["--root", "--manifest", "--head", "--expected-head"]);
  for (const key of values.keys()) {
    if (!allowed.has(key)) fail(`unknown option ${key}`);
  }
  const root = values.get("--root");
  const manifest = values.get("--manifest");
  const head = command === "seal" ? values.get("--head") : values.get("--expected-head");
  if (!root || !manifest || !head || !HEAD_PATTERN.test(head)) {
    fail(`${command} requires --root, --manifest, and a 40-character lowercase HEAD`);
  }
  return { command, root, manifest, head };
}

function canonicalRelative(root, absolute) {
  const relative = path.relative(root, absolute).split(path.sep).join("/");
  if (!relative || relative.startsWith("../") || path.isAbsolute(relative)) {
    fail(`path escapes Web dist root: ${relative || "<root>"}`);
  }
  return relative;
}

async function collectFiles(root) {
  const rootStat = await lstat(root).catch(() => null);
  if (!rootStat?.isDirectory() || rootStat.isSymbolicLink()) {
    fail("Web dist root must be a real directory");
  }
  const canonicalRoot = await realpath(root);
  const pending = [canonicalRoot];
  const files = [];
  let totalBytes = 0;
  while (pending.length > 0) {
    const directory = pending.pop();
    const entries = await readdir(directory, { withFileTypes: true });
    entries.sort((left, right) => compareCanonicalPath(left.name, right.name));
    for (const entry of entries) {
      const absolute = path.join(directory, entry.name);
      const stat = await lstat(absolute);
      if (stat.isSymbolicLink()) fail(`symlink is forbidden: ${canonicalRelative(canonicalRoot, absolute)}`);
      if (stat.isDirectory()) {
        pending.push(absolute);
        continue;
      }
      if (!stat.isFile()) fail(`unsupported entry: ${canonicalRelative(canonicalRoot, absolute)}`);
      const bytes = await readFile(absolute);
      if (bytes.byteLength > MAX_FILE_BYTES) fail(`file exceeds ${MAX_FILE_BYTES} bytes: ${entry.name}`);
      totalBytes += bytes.byteLength;
      if (totalBytes > MAX_TOTAL_BYTES) fail(`Web dist exceeds ${MAX_TOTAL_BYTES} bytes`);
      files.push({
        path: canonicalRelative(canonicalRoot, absolute),
        size: bytes.byteLength,
        sha256: createHash("sha256").update(bytes).digest("hex"),
      });
      if (files.length > MAX_FILES) fail(`Web dist exceeds ${MAX_FILES} files`);
    }
  }
  files.sort((left, right) => compareCanonicalPath(left.path, right.path));
  if (files.length === 0) fail("Web dist is empty");
  return files;
}

function canonicalTreeDigest(files) {
  const hash = createHash("sha256");
  for (const file of files) hash.update(`${file.sha256} ${file.size} ${file.path}\n`, "utf8");
  return hash.digest("hex");
}

function validateManifest(manifest, expectedHead) {
  if (
    manifest?.schema !== 1 ||
    manifest?.kind !== "deve-web-dist" ||
    manifest?.head !== expectedHead ||
    !Array.isArray(manifest?.files) ||
    !SHA256_PATTERN.test(manifest?.tree_sha256 ?? "")
  ) {
    fail("manifest identity or schema mismatch");
  }
  let previous = "";
  let totalBytes = 0;
  for (const file of manifest.files) {
    if (
      typeof file?.path !== "string" ||
      !file.path ||
      file.path.includes("\\") ||
      file.path.startsWith("/") ||
      file.path.split("/").some((part) => !part || part === "." || part === "..") ||
      !Number.isSafeInteger(file?.size) ||
      file.size < 0 ||
      file.size > MAX_FILE_BYTES ||
      !SHA256_PATTERN.test(file?.sha256 ?? "") ||
      (previous && compareCanonicalPath(file.path, previous) <= 0)
    ) {
      fail("manifest contains a noncanonical file entry");
    }
    totalBytes += file.size;
    if (!Number.isSafeInteger(totalBytes) || totalBytes > MAX_TOTAL_BYTES) {
      fail("manifest total size is out of bounds");
    }
    previous = file.path;
  }
  if (manifest.files.length === 0 || manifest.files.length > MAX_FILES) {
    fail("manifest file count is out of bounds");
  }
}

async function main() {
  const { command, root, manifest, head } = parseArgs(process.argv.slice(2));
  const files = await collectFiles(root);
  const treeSha256 = canonicalTreeDigest(files);
  if (command === "seal") {
    const document = { schema: 1, kind: "deve-web-dist", head, tree_sha256: treeSha256, files };
    await mkdir(path.dirname(manifest), { recursive: true });
    await writeFile(manifest, `${JSON.stringify(document, null, 2)}\n`, { flag: "wx" });
    process.stdout.write(`web-dist-artifact: sealed ${files.length} files ${treeSha256}\n`);
    return;
  }
  const document = JSON.parse(await readFile(manifest, "utf8"));
  validateManifest(document, head);
  if (JSON.stringify(document.files) !== JSON.stringify(files) || document.tree_sha256 !== treeSha256) {
    fail("Web dist bytes do not match the immutable manifest");
  }
  process.stdout.write(`web-dist-artifact: verified ${files.length} files ${treeSha256}\n`);
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
});

import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("./web-dist-artifact.mjs", import.meta.url));
const head = "a".repeat(40);

function fixture() {
  const root = mkdtempSync(path.join(tmpdir(), "deve-web-dist-"));
  const dist = path.join(root, "dist");
  const manifest = path.join(root, "manifest.json");
  mkdirSync(path.join(dist, "assets"), { recursive: true });
  writeFileSync(path.join(dist, "index.html"), "<!doctype html>\n");
  writeFileSync(path.join(dist, "assets", "app.js"), "export {};\n");
  return { dist, manifest };
}

function run(...args) {
  return execFileSync(process.execPath, [script, ...args], { encoding: "utf8" });
}

test("seal and verify bind sorted bytes to the exact head", () => {
  const { dist, manifest } = fixture();
  assert.match(run("seal", "--root", dist, "--manifest", manifest, "--head", head), /sealed 2 files/);
  const document = JSON.parse(readFileSync(manifest, "utf8"));
  assert.equal(document.head, head);
  assert.deepEqual(document.files.map((file) => file.path), ["assets/app.js", "index.html"]);
  assert.match(run("verify", "--root", dist, "--manifest", manifest, "--expected-head", head), /verified 2 files/);
});

test("verify rejects changed bytes and a different head", () => {
  const { dist, manifest } = fixture();
  run("seal", "--root", dist, "--manifest", manifest, "--head", head);
  writeFileSync(path.join(dist, "index.html"), "changed\n");
  assert.notEqual(spawnSync(process.execPath, [script, "verify", "--root", dist, "--manifest", manifest, "--expected-head", head]).status, 0);
  assert.notEqual(spawnSync(process.execPath, [script, "verify", "--root", dist, "--manifest", manifest, "--expected-head", "b".repeat(40)]).status, 0);
});

test("verify rejects manifest sizes outside the bounded artifact contract", () => {
  const { dist, manifest } = fixture();
  run("seal", "--root", dist, "--manifest", manifest, "--head", head);
  const document = JSON.parse(readFileSync(manifest, "utf8"));
  document.files[0].size = 32 * 1024 * 1024 + 1;
  writeFileSync(manifest, `${JSON.stringify(document)}\n`);
  assert.notEqual(
    spawnSync(process.execPath, [
      script,
      "verify",
      "--root",
      dist,
      "--manifest",
      manifest,
      "--expected-head",
      head,
    ]).status,
    0,
  );
});

test("seal and verify share UTF-8 bytewise ordering for real Trunk-style and Unicode names", () => {
  const { dist, manifest } = fixture();
  for (const name of [
    "-a.js",
    "A.js",
    "_a.js",
    "a.js",
    "deve_web-abc.js",
    "deve_web-abc_bg.wasm",
    "\uE000.js",
    "𐀀.js",
  ]) {
    writeFileSync(path.join(dist, name), name);
  }
  run("seal", "--root", dist, "--manifest", manifest, "--head", head);
  const document = JSON.parse(readFileSync(manifest, "utf8"));
  const paths = document.files.map((file) => file.path);
  assert.deepEqual(paths, [...paths].sort((left, right) => Buffer.compare(Buffer.from(left), Buffer.from(right))));
  assert.ok(paths.indexOf("deve_web-abc.js") < paths.indexOf("deve_web-abc_bg.wasm"));
  assert.ok(paths.indexOf("\uE000.js") < paths.indexOf("𐀀.js"));
  const expectedCount = process.platform === "win32" ? 9 : 10;
  assert.match(
    run("verify", "--root", dist, "--manifest", manifest, "--expected-head", head),
    new RegExp(`verified ${expectedCount} files`),
  );
});

test("seal rejects symlinks instead of following external bytes", { skip: process.platform === "win32" }, () => {
  const { dist, manifest } = fixture();
  symlinkSync(path.join(dist, "index.html"), path.join(dist, "alias.html"));
  assert.notEqual(spawnSync(process.execPath, [script, "seal", "--root", dist, "--manifest", manifest, "--head", head]).status, 0);
});

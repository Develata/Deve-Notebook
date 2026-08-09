import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { EventEmitter } from "node:events";
import test from "node:test";
import {
  androidLogcatContains,
  androidLogcatMatchStates,
} from "./lib/android-logcat-observation.mjs";

function fixtureSpawn(source, observed) {
  return (program, args, options) => {
    const child = spawn(process.execPath, ["-e", source], options);
    observed.push({ program, args, options, child });
    return child;
  };
}

function inertChild({ killResult = false } = {}) {
  const child = new EventEmitter();
  child.stdout = new EventEmitter();
  child.stderr = new EventEmitter();
  child.killCalls = [];
  child.kill = (signal) => {
    child.killCalls.push(signal);
    return killResult;
  };
  return child;
}

function probe(source, overrides = {}) {
  const observed = [];
  return {
    observed,
    result: androidLogcatContains({
      adb: "/synthetic/adb",
      serial: "emulator-5584",
      pattern: /deve_mobile .*LocalBackend/,
      timeoutMs: 5_000,
      maxOutputBytes: 4 * 1024 * 1024,
      spawnProcess: fixtureSpawn(source, observed),
      ...overrides,
    }),
  };
}

test("Android logcat observation streams past the execFileSync default buffer", async () => {
  const noiseBytes = 240_000 * Buffer.byteLength("noise\n");
  assert.ok(noiseBytes > 1024 * 1024);
  const source = `
    for (let index = 0; index < 240000; index += 1) {
      process.stdout.write("noise\\n");
    }
    process.stdout.write("deve_mobile preflight LocalBackend marker\\n");
  `;
  const { observed, result } = probe(source);

  assert.equal(await result, true);
  assert.equal(observed.length, 1);
  assert.equal(observed[0].program, "/synthetic/adb");
  assert.deepEqual(observed[0].args, [
    "-s",
    "emulator-5584",
    "logcat",
    "-d",
    "-v",
    "raw",
  ]);
  assert.equal(observed[0].options.stdio[0], "ignore");
});

test("Android logcat observation returns false when the marker is absent", async () => {
  const { result } = probe(`process.stdout.write("ordinary log line\\n");`);
  assert.equal(await result, false);
});

test("Android logcat mode evidence ignores LocalBackend wording outside exact runtime markers", async () => {
  const observed = [];
  const matches = await androidLogcatMatchStates({
    adb: "/synthetic/adb",
    serial: "emulator-5584",
    patterns: [
      /^deve_mobile native shell mode=RemoteBrowser embedded_backend=absent$/,
      /^deve_mobile native embedded backend supervisor=started$/,
    ],
    timeoutMs: 5_000,
    spawnProcess: fixtureSpawn(`
      process.stdout.write("deve_mobile native LocalBackend recovery control failed closed: synthetic\\n");
      process.stdout.write("deve_mobile native shell mode=RemoteBrowser embedded_backend=absent\\n");
    `, observed),
  });

  assert.deepEqual(matches, [true, false]);
  assert.equal(observed.length, 1, "mode ownership evidence must come from one log snapshot");
});

test("Android logcat mode evidence recognizes the forbidden supervisor marker", async () => {
  const observed = [];
  const matches = await androidLogcatMatchStates({
    adb: "/synthetic/adb",
    serial: "emulator-5584",
    patterns: [
      /^deve_mobile native shell mode=RemoteBrowser embedded_backend=absent$/,
      /^deve_mobile native embedded backend supervisor=started$/,
    ],
    timeoutMs: 5_000,
    spawnProcess: fixtureSpawn(`
      process.stdout.write("deve_mobile native shell mode=RemoteBrowser embedded_backend=absent\\n");
      process.stdout.write("deve_mobile native embedded backend supervisor=started\\n");
    `, observed),
  });

  assert.deepEqual(matches, [true, true]);
  assert.equal(observed.length, 1);
});

test("Android logcat observation returns false after large marker-free output", async () => {
  const { result } = probe(`
    for (let index = 0; index < 240000; index += 1) {
      process.stdout.write("noise\\n");
    }
  `);
  assert.equal(await result, false);
});

test("Android logcat observation matches markers split across output chunks", async () => {
  const source = `
    process.stdout.write("deve_mobile recovery Local");
    setTimeout(() => process.stdout.write("Backend ready\\n"), 20);
  `;
  const { result } = probe(source);
  assert.equal(await result, true);
});

test("Android logcat observation fails closed on command errors", async () => {
  const { result } = probe(`
    process.stderr.write("synthetic adb transport failure\\n");
    process.exit(7);
  `);
  await assert.rejects(result, /exited with status 7.*synthetic adb transport failure/s);
});

test("Android logcat observation rejects a marker from a failed command", async () => {
  const { result } = probe(`
    process.stdout.write("deve_mobile preflight LocalBackend marker\\n");
    process.exit(7);
  `);
  await assert.rejects(result, /exited with status 7/);
});

test("Android logcat observation fails closed on synchronous spawn errors", async () => {
  const { result } = probe("", {
    spawnProcess: () => { throw new Error("synthetic spawn failure"); },
  });
  await assert.rejects(result, /failed to start: synthetic spawn failure/);
});

test("Android logcat observation fails closed on asynchronous spawn errors", async () => {
  const child = inertChild();
  const { result } = probe("", { spawnProcess: () => child });
  queueMicrotask(() => child.emit("error", new Error("synthetic async ENOENT")));
  await assert.rejects(result, /failed to start: synthetic async ENOENT/);
});

test("Android logcat observation fails closed on timeout", async () => {
  const { observed, result } = probe(`setInterval(() => {}, 1000);`, {
    timeoutMs: 50,
  });
  await assert.rejects(result, /timed out after 50 ms/);
  assert.equal(observed[0].child.killed, true);
});

test("Android logcat timeout stays bounded when the child never closes", async () => {
  const child = inertChild();
  const startedAt = Date.now();
  const { result } = probe("", {
    timeoutMs: 20,
    terminationGraceMs: 20,
    spawnProcess: () => child,
  });
  await assert.rejects(
    result,
    /timed out after 20 ms; child termination was not accepted; child did not close within 20 ms/,
  );
  assert.deepEqual(child.killCalls, ["SIGKILL"]);
  assert.ok(Date.now() - startedAt < 500);
});

test("Android logcat failure waits for bounded child retirement after a late error", async () => {
  const child = inertChild();
  const startedAt = Date.now();
  const { result } = probe("", {
    timeoutMs: 20,
    terminationGraceMs: 40,
    spawnProcess: () => child,
  });
  setTimeout(() => child.emit("error", new Error("synthetic post-timeout child error")), 30);

  await assert.rejects(
    result,
    /timed out after 20 ms; child termination was not accepted; child did not close within 40 ms/,
  );
  assert.ok(Date.now() - startedAt >= 45, "late child error must not bypass termination grace");
});

test("Android logcat observation fails closed on total output overflow", async () => {
  const { result } = probe(`process.stdout.write("x".repeat(8192));`, {
    maxOutputBytes: 4096,
    maxLineBytes: 4096,
  });
  await assert.rejects(result, /exceeded the 4096 byte output limit/);
});

test("Android logcat observation fails closed on an oversized line", async () => {
  const { result } = probe(`process.stdout.write("x".repeat(2048));`, {
    maxLineBytes: 1024,
  });
  await assert.rejects(result, /line exceeded the 1024 byte limit/);
});

test("Android logcat observation rejects an oversized complete line", async () => {
  const { result } = probe(`
    process.stdout.write("x".repeat(2048) + "deve_mobile marker LocalBackend\\n");
  `, { maxLineBytes: 1024 });
  await assert.rejects(result, /line exceeded the 1024 byte limit/);
});

import assert from "node:assert/strict";
import test from "node:test";
import {
  observeAnchoredAndroidAppProcess,
  probeAndroidAppProcess,
  waitForAnchoredAndroidAppProcessExit,
} from "./lib/android-app-process-observation.mjs";

test("Android pidof probe distinguishes ordinary absence from probe failure", () => {
  const configuration = {
    adb: "adb",
    serial: "emulator-5554",
    appId: "dev.deve.notebook.mobile",
    timeoutMs: 5000,
  };
  const absent = Object.assign(new Error("absent"), { status: 1, stdout: "", stderr: "" });
  assert.equal(probeAndroidAppProcess(configuration, () => { throw absent; }), "");

  const transport = Object.assign(new Error("transport"), {
    status: 1,
    stdout: "",
    stderr: "error: device offline",
  });
  assert.throws(
    () => probeAndroidAppProcess(configuration, () => { throw transport; }),
    /android_app_process_probe_failed.*device offline/,
  );
  assert.throws(
    () => probeAndroidAppProcess(configuration, () => { throw new Error("timeout"); }),
    /android_app_process_probe_failed/,
  );
});

test("Android pidof probe uses argv and returns the first canonical PID", () => {
  let invocation;
  const pid = probeAndroidAppProcess(
    {
      adb: "adb-custom",
      serial: "emulator-5556",
      appId: "dev.deve.notebook.mobile",
      timeoutMs: 1234,
    },
    (...args) => {
      invocation = args;
      return "4066 4067\r\n";
    },
  );
  assert.equal(pid, "4066");
  assert.deepEqual(invocation, [
    "adb-custom",
    ["-s", "emulator-5556", "shell", "pidof", "dev.deve.notebook.mobile"],
    { encoding: "utf8", timeout: 1234 },
  ]);
});

test("anchored Android process observation tolerates one bookkeeping gap", async () => {
  const samples = ["", "4066"];
  const delays = [];
  const pid = await observeAnchoredAndroidAppProcess("4066", {
    probe: async () => samples.shift(),
    delay: async (milliseconds) => delays.push(milliseconds),
  });
  assert.equal(pid, "4066");
  assert.deepEqual(delays, [1000]);
  assert.equal(samples.length, 0);
});

test("anchored Android process observation samples twice before continued-absence failure", async () => {
  let probes = 0;
  const delays = [];
  await assert.rejects(
    observeAnchoredAndroidAppProcess("4066", {
      probe: async () => {
        probes += 1;
        return "";
      },
      delay: async (milliseconds) => delays.push(milliseconds),
    }),
    /android_app_process_absent_after_admission.*4066/,
  );
  assert.equal(probes, 2);
  assert.deepEqual(delays, [1000]);
});

test("anchored Android process observation rejects replacement and probe error", async () => {
  const replacementSamples = ["", "5000"];
  await assert.rejects(
    observeAnchoredAndroidAppProcess("4066", {
      probe: async () => replacementSamples.shift(),
      delay: async () => {},
    }),
    /android_app_process_replaced.*4066.*5000/,
  );
  assert.equal(replacementSamples.length, 0);
  await assert.rejects(
    observeAnchoredAndroidAppProcess("4066", {
      probe: async () => { throw new Error("probe offline"); },
      delay: async () => {},
    }),
    /probe offline/,
  );
});

test("Android graceful exit requires consecutive absence and resets after recovery", async () => {
  const samples = ["4066", "", "4066", "", ""];
  let clock = 0;
  const exited = await waitForAnchoredAndroidAppProcessExit("4066", {
    probe: async () => samples.shift(),
    delay: async (milliseconds) => { clock += milliseconds; },
    now: () => clock,
    timeoutMs: 5000,
  });
  assert.equal(exited, true);
  assert.equal(samples.length, 0);
});

test("Android graceful exit rejects replacement and probe error", async () => {
  await assert.rejects(
    waitForAnchoredAndroidAppProcessExit("4066", {
      probe: async () => "5000",
      delay: async () => {},
      now: () => 0,
      timeoutMs: 5000,
    }),
    /android_app_process_replaced.*4066.*5000/,
  );
  await assert.rejects(
    waitForAnchoredAndroidAppProcessExit("4066", {
      probe: async () => { throw new Error("probe offline"); },
      delay: async () => {},
      now: () => 0,
      timeoutMs: 5000,
    }),
    /probe offline/,
  );
});

test("Android graceful exit rejects a sample that returns after its deadline", async () => {
  let clock = 0;
  await assert.rejects(
    waitForAnchoredAndroidAppProcessExit("4066", {
      probe: async (remaining) => {
        assert.equal(remaining, 500);
        clock = 500;
        return "";
      },
      delay: async () => {},
      now: () => clock,
      timeoutMs: 500,
    }),
    /android_app_process_exit_timeout.*4066/,
  );
});

test("Android process observation rejects invalid anchors, output, and timeout", async () => {
  await assert.rejects(
    observeAnchoredAndroidAppProcess("", { probe: async () => "", delay: async () => {} }),
    /android_app_process_anchor_invalid/,
  );
  await assert.rejects(
    observeAnchoredAndroidAppProcess("4066", {
      probe: async () => "not-a-pid",
      delay: async () => {},
    }),
    /android_app_process_probe_invalid/,
  );
  await assert.rejects(
    waitForAnchoredAndroidAppProcessExit("4066", {
      probe: async () => "",
      delay: async () => {},
      now: () => 0,
      timeoutMs: 0,
    }),
    /android_app_process_exit_timeout_invalid/,
  );
});

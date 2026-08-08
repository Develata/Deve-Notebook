import { execFileSync } from "node:child_process";

function canonicalPid(value) {
  const pid = String(value ?? "")
    .replaceAll("\r", "")
    .trim()
    .split(/\s+/)
    .filter(Boolean)[0] ?? "";
  if (pid && !/^[1-9][0-9]*$/.test(pid)) {
    throw new Error(`android_app_process_probe_invalid: ${JSON.stringify({ pid })}`);
  }
  return pid;
}

function boundedDiagnostic(value) {
  return String(value ?? "").replaceAll("\r", "").trim().slice(0, 512);
}

export function probeAndroidAppProcess(
  { adb, serial, appId, timeoutMs },
  execFile = execFileSync,
) {
  if (!adb || !serial || !appId || !Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    throw new Error("android_app_process_probe_configuration_invalid");
  }
  try {
    const output = execFile(
      adb,
      ["-s", serial, "shell", "pidof", appId],
      { encoding: "utf8", timeout: Math.max(1, Math.floor(timeoutMs)) },
    );
    return canonicalPid(output);
  } catch (error) {
    const status = Number.isInteger(error?.status) ? error.status : null;
    const stdout = boundedDiagnostic(error?.stdout);
    const stderr = boundedDiagnostic(error?.stderr);
    if (status === 1 && !stdout && !stderr) return "";
    throw new Error(
      `android_app_process_probe_failed: ${JSON.stringify({
        status,
        signal: error?.signal ?? null,
        stdout,
        stderr,
      })}`,
    );
  }
}

function requireExpectedPid(expectedPid) {
  const pid = canonicalPid(expectedPid);
  if (!pid) throw new Error("android_app_process_anchor_invalid");
  return pid;
}

function replacementError(expectedPid, currentPid) {
  return new Error(
    `android_app_process_replaced: ${JSON.stringify({
      initialPid: expectedPid,
      currentPid,
    })}`,
  );
}

export async function observeAnchoredAndroidAppProcess(
  expectedPid,
  {
    probe,
    delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)),
  },
) {
  const anchoredPid = requireExpectedPid(expectedPid);
  for (let missingSamples = 0; missingSamples < 2; missingSamples += 1) {
    const observedPid = canonicalPid(await probe());
    if (observedPid === anchoredPid) return anchoredPid;
    if (observedPid) throw replacementError(anchoredPid, observedPid);
    if (missingSamples === 0) await delay(1000);
  }
  throw new Error(
    `android_app_process_absent_after_admission: ${JSON.stringify({
      pid: anchoredPid,
      missingSamples: 2,
    })}`,
  );
}

export async function waitForAnchoredAndroidAppProcessExit(
  expectedPid,
  {
    probe,
    delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)),
    now = Date.now,
    timeoutMs = 30000,
  },
) {
  const anchoredPid = requireExpectedPid(expectedPid);
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    throw new Error("android_app_process_exit_timeout_invalid");
  }
  const stop = now() + timeoutMs;
  let missingSamples = 0;
  while (true) {
    const remainingBeforeProbe = stop - now();
    if (remainingBeforeProbe <= 0) break;
    const observedPid = canonicalPid(await probe(Math.max(1, remainingBeforeProbe)));
    if (now() >= stop) {
      throw new Error(
        `android_app_process_exit_timeout: ${JSON.stringify({ pid: anchoredPid })}`,
      );
    }
    if (!observedPid) {
      missingSamples += 1;
      if (missingSamples >= 2) return true;
    } else if (observedPid !== anchoredPid) {
      throw replacementError(anchoredPid, observedPid);
    } else {
      missingSamples = 0;
    }
    const remaining = stop - now();
    if (remaining > 0) await delay(Math.min(1000, remaining));
  }
  throw new Error(
    `android_app_process_exit_timeout: ${JSON.stringify({ pid: anchoredPid })}`,
  );
}

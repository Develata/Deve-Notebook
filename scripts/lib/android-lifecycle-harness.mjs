import { execFileSync } from "node:child_process";

export function createAndroidLifecycleHarness({ timeoutMs, adb, serial }) {
  const harnessDeadline = Date.now() + timeoutMs;

  function remainingMs() {
    const remaining = harnessDeadline - Date.now();
    if (remaining <= 0) throw new Error("Android lifecycle harness deadline exhausted");
    return remaining;
  }

  function delay(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  async function withDeadline(label, promise, limit = remainingMs()) {
    let timer;
    try {
      return await Promise.race([
        promise,
        new Promise((_, reject) => {
          timer = setTimeout(
            () => reject(new Error(`timeout during ${label}`)),
            Math.min(limit, remainingMs()),
          );
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
  }

  async function waitUntil(label, predicate, timeout = Math.min(timeoutMs, 30000)) {
    const deadline = Math.min(Date.now() + timeout, harnessDeadline);
    let lastError;
    while (Date.now() < deadline) {
      try {
        const value = await withDeadline(
          `${label} predicate`,
          Promise.resolve().then(predicate),
          Math.max(1, deadline - Date.now()),
        );
        if (value) return value;
      } catch (error) {
        lastError = error;
      }
      await delay(Math.min(250, Math.max(1, deadline - Date.now())));
    }
    throw new Error(`timeout waiting for ${label}${lastError ? `: ${lastError.message}` : ""}`);
  }

  function adbCommand(...args) {
    execFileSync(adb, ["-s", serial, ...args], {
      stdio: "inherit",
      timeout: remainingMs(),
    });
  }

  function adbOutput(...args) {
    return execFileSync(adb, ["-s", serial, ...args], {
      encoding: "utf8",
      timeout: remainingMs(),
    }).replaceAll("\r", "");
  }

  async function tapAndroidEditor(point) {
    if (!Number.isFinite(point?.x)
      || !Number.isFinite(point?.y)
      || !Number.isFinite(point?.devicePixelRatio)
      || point.devicePixelRatio <= 0) {
      throw new Error("Android editor native tap point is invalid");
    }
    adbCommand(
      "shell", "input", "tap",
      String(Math.round(point.x * point.devicePixelRatio)),
      String(Math.round(point.y * point.devicePixelRatio)),
    );
    await delay(250);
  }

  async function proveAndroidRootBackBackground(appId, reentryReady) {
    const readPid = () => adbOutput("shell", "pidof", appId).trim();
    const isResumed = () => adbOutput("shell", "dumpsys", "activity", "activities")
      .split("\n")
      .some((line) => line.includes("mResumedActivity") && line.includes(appId));
    const rootBackLogCount = () => adbOutput(
      "shell", "logcat", "-d", "-s", "DeveMobile:I", "*:S",
    ).split("\n").filter((line) => line.includes("android_ui_back_root_backgrounded")).length;

    const pidBefore = readPid();
    if (!pidBefore) throw new Error("Android root Back proof requires a live app PID");
    const logsBefore = rootBackLogCount();
    adbCommand("shell", "input", "keyevent", "4");
    await waitUntil("root Back backgrounds task", () => (
      !isResumed() && readPid() === pidBefore && rootBackLogCount() > logsBefore
    ), 30000);
    adbCommand("shell", "monkey", "-p", appId, "-c", "android.intent.category.LAUNCHER", "1");
    await waitUntil("root Back task reentry", () => isResumed() && readPid() === pidBefore, 30000);
    await reentryReady();
    return { pidStable: readPid() === pidBefore, rootBackBackgrounded: true, reentryReady: true };
  }

  async function waitForAndroidRootReentry(readState, presentationBeforeRootBack) {
    return waitUntil("root Back lifecycle rebind", async () => {
      const { state, projection, presentation } = await readState();
      return state?.backend_running === true
        && state.service_state === "endpoint_session_ready"
        && projection.syncStatus === "handshaking-repo"
        && projection.repoIdRaw === ""
        && projection.loginVisible === false
        && projection.bootstrapSessionBound
        && projection.nativeSessionInstalled
        && presentation?.generation === presentationBeforeRootBack.generation
        && Number.isSafeInteger(presentation?.epoch)
        && presentation.epoch > presentationBeforeRootBack.epoch;
    }, 120000);
  }

  return {
    remainingMs,
    delay,
    withDeadline,
    waitUntil,
    adbCommand,
    adbOutput,
    tapAndroidEditor,
    proveAndroidRootBackBackground,
    waitForAndroidRootReentry,
  };
}

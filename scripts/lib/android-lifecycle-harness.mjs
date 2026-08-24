import { execFileSync } from "node:child_process";

const ROOT_REENTRY_SAMPLE_TIMEOUT_MS = 5_000;

function rootReentryObservation(raw, presentationBeforeRootBack) {
  const state = raw?.state;
  const projection = raw?.projection;
  const presentation = raw?.presentation;
  const generation = Number.isSafeInteger(presentation?.generation)
    ? presentation.generation
    : null;
  const epoch = Number.isSafeInteger(presentation?.epoch) ? presentation.epoch : null;
  return {
    backendRunning: state?.backend_running === true,
    endpointSessionReady: state?.service_state === "endpoint_session_ready",
    bootstrapUnbound: projection?.syncStatus === "handshaking-repo"
      && projection.repoIdRaw === "",
    loginHidden: projection?.loginVisible === false,
    bootstrapSessionBound: projection?.bootstrapSessionBound === true,
    nativeSessionInstalled: projection?.nativeSessionInstalled === true,
    samePresentationGeneration: generation === presentationBeforeRootBack.generation,
    freshPresentationEpoch: epoch !== null && epoch > presentationBeforeRootBack.epoch,
    presentationGeneration: generation,
    presentationEpoch: epoch,
  };
}

function rootReentryObservationIsReady(observation) {
  return observation.backendRunning
    && observation.endpointSessionReady
    && observation.bootstrapUnbound
    && observation.loginHidden
    && observation.bootstrapSessionBound
    && observation.nativeSessionInstalled
    && observation.samePresentationGeneration
    && observation.freshPresentationEpoch;
}

export function requireAndroidRootBackStablePid(pidBefore, pidAfter) {
  if (!/^[1-9][0-9]*$/.test(pidBefore)
    || !/^[1-9][0-9]*$/.test(pidAfter)
    || pidAfter !== pidBefore) {
    throw new Error("android_root_back_pid_unstable");
  }
  return true;
}

export function classifyAndroidActivityResumed(output, appId) {
  if (typeof output !== "string"
    || typeof appId !== "string"
    || !/^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$/.test(appId)) {
    return "unavailable";
  }
  const records = output.replaceAll("\r", "").split("\n").filter((line) => {
    const normalized = line.trim();
    return /^(?:mResumedActivity|topResumedActivity)\s*[:=]/.test(normalized)
      || /^ResumedActivity\s*:/.test(normalized);
  });
  if (records.length === 0) return "unavailable";
  const components = records.map((line) => {
    const activityRecords = [...line.matchAll(/ActivityRecord\{([^{}]+)\}/g)];
    if (activityRecords.length !== 1) return null;
    const matches = [...activityRecords[0][1].matchAll(
      /(?:^|[\s{])([A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+)\/([A-Za-z0-9_.$]+)(?=[\s}])/g,
    )];
    return matches.length === 1 ? `${matches[0][1]}/${matches[0][2]}` : null;
  });
  if (components.some((component) => component === null)) return "unavailable";
  const uniqueComponents = new Set(components);
  if (uniqueComponents.size !== 1) return "unavailable";
  const [component] = uniqueComponents;
  const packageName = component.slice(0, component.indexOf("/"));
  if (packageName === appId) return "resumed";
  if (packageName.startsWith(`${appId}.`) || appId.startsWith(`${packageName}.`)) {
    return "unavailable";
  }
  return "not-resumed";
}

export function createAndroidLifecycleHarness({
  timeoutMs,
  adb,
  serial,
  rootReentrySampleTimeoutMs = ROOT_REENTRY_SAMPLE_TIMEOUT_MS,
}) {
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
    try {
      const readPid = () => adbOutput("shell", "pidof", appId).trim();
      const resumedState = () => classifyAndroidActivityResumed(
        adbOutput("shell", "dumpsys", "activity", "activities"),
        appId,
      );
      const rootBackLogCount = () => adbOutput(
        "shell", "logcat", "-d", "-s", "DeveMobile:I", "*:S",
      ).split("\n").filter((line) => line.includes("android_ui_back_root_backgrounded")).length;

      const pidBefore = readPid();
      requireAndroidRootBackStablePid(pidBefore, pidBefore);
      const logsBefore = rootBackLogCount();
      adbCommand("shell", "input", "keyevent", "4");
      await waitUntil("root Back backgrounds task", () => (
        resumedState() === "not-resumed"
          && readPid() === pidBefore
          && rootBackLogCount() > logsBefore
      ), 30000);
      adbCommand(
        "shell", "monkey", "-p", appId, "-c", "android.intent.category.LAUNCHER", "1",
      );
      await waitUntil("root Back task reentry", () => (
        resumedState() === "resumed" && readPid() === pidBefore
      ), 30000);
      await reentryReady();
      requireAndroidRootBackStablePid(pidBefore, readPid());
      return { pidStable: true, rootBackBackgrounded: true, reentryReady: true };
    } catch {
      throw new Error("android_root_back_proof_failed");
    }
  }

  async function waitForAndroidRootReentry(readState, presentationBeforeRootBack) {
    const sampleBudget = Number.isSafeInteger(rootReentrySampleTimeoutMs)
      && rootReentrySampleTimeoutMs > 0
      ? rootReentrySampleTimeoutMs
      : ROOT_REENTRY_SAMPLE_TIMEOUT_MS;
    let lastObservation = null;
    let sampleFailures = 0;
    try {
      return await waitUntil("root Back lifecycle rebind", async () => {
        let raw;
        try {
          // Resolve the remaining budget before starting the readonly sample. If
          // the harness deadline has already elapsed, invoking readState first
          // would leave its rejection without the bounded diagnostic wrapper.
          const sampleLimit = Math.min(sampleBudget, remainingMs());
          raw = await withDeadline(
            "Android root Back reentry readonly sample",
            Promise.resolve().then(readState),
            sampleLimit,
          );
        } catch {
          sampleFailures += 1;
          return false;
        }
        lastObservation = rootReentryObservation(raw, presentationBeforeRootBack);
        return rootReentryObservationIsReady(lastObservation);
      }, 120000);
    } catch {
      const category = sampleFailures > 0
        ? "android_root_reentry_sample_failed"
        : "android_root_reentry_incomplete";
      throw new Error(
        `root Back lifecycle rebind failed; category=${category}; `
          + `sampleFailures=${sampleFailures}; last=${JSON.stringify(lastObservation)}`,
      );
    }
  }

  async function readAndroidUiBackSurfaceObservation(page) {
    try {
      return await page.call(() => {
        const visible = (selector) => [...document.querySelectorAll(selector)].some((element) => {
          const style = getComputedStyle(element);
          const rect = element.getBoundingClientRect();
          return style.display !== "none"
            && style.visibility !== "hidden"
            && rect.width > 0
            && rect.height > 0;
        });
        return {
          editorVisible: visible("[data-deve-editor-host=true]"),
          repoSwitcherVisible: visible("[data-deve-repo-switcher-menu]"),
          repoRemovalDialogVisible: visible('[data-deve-repo-removal-dialog="visible"]'),
          settingsVisible: visible('[data-deve-settings-surface="modal"]'),
          searchVisible: visible("[data-deve-search-sheet-position]"),
          mobileMoreMenuVisible: visible("[data-deve-mobile-more-menu]"),
          sourceControlMenuVisible: visible('[data-deve-sc-section-menu="true"]'),
          surfaceSwitcherVisible: visible('[data-deve-mobile-surface-sheet="open"]'),
          chatExpanded: visible('[data-deve-mobile-chat="expanded"]'),
          leftDrawerOpen: document.querySelector('[data-deve-mobile-drawer="left"]')
            ?.getAttribute("data-deve-mobile-drawer-open") === "true",
          rightDrawerOpen: document.querySelector('[data-deve-mobile-drawer="right"]')
            ?.getAttribute("data-deve-mobile-drawer-open") === "true",
          visibleDialogCount: [...document.querySelectorAll('[role="dialog"]')]
            .filter((element) => {
              const style = getComputedStyle(element);
              const rect = element.getBoundingClientRect();
              return style.display !== "none"
                && style.visibility !== "hidden"
                && rect.width > 0
                && rect.height > 0;
            }).length,
        };
      });
    } catch {
      return {
        observationAvailable: false,
        category: "android_ui_back_surface_observation_failed",
      };
    }
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
    readAndroidUiBackSurfaceObservation,
  };
}

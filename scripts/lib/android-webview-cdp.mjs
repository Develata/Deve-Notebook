import {
  CdpPage,
  DISCOVERY_COMMAND_TIMEOUT_MS,
  visibleElement,
} from "./android-webview-cdp-client.mjs";

export {
  CdpPage,
  isExpectedCdpTargetRetirement,
  visibleElement,
} from "./android-webview-cdp-client.mjs";

const STABLE_PAGE_TIMEOUT_MS = 60_000;
const PAGE_GENERATION_TIMEOUT_MS = 30_000;
const DISCOVERY_POLL_INTERVAL_MS = 250;
const REQUIRED_PAGE_SURFACES = new Set(["sync", "remote-entry"]);
const DOCUMENT_READY_STATES = new Set(["loading", "interactive", "complete"]);
const SYNC_STATUS_VALUES = new Set([
  "session-expired",
  "native-bootstrap-invalid",
  "native-session-pending",
  "native-service-offline",
  "native-reprobe-required",
  "offline",
  "reconnecting",
  "snapshot-loading",
  "editor-sync-error",
  "read-only",
  "handshaking-repo",
  "peer-not-registered",
  "pending-ack",
  "ready",
]);
const NATIVE_SERVICE_STATE_VALUES = new Set([
  "endpoint_session_ready",
  "background_suspended",
  "foreground_reprobe",
  "stopping",
  "stopped",
  "error",
  "service_offline",
  "session_invalid",
  "endpoint_ready",
  "session_bound",
  "runtime_ready",
]);
const NATIVE_BLOCKED_REASON_VALUES = new Set(["session_invalid"]);

function readPageSnapshot() {
  const status = document.querySelector("[data-deve-sync-status]");
  const bootstrap = globalThis.__DEVE_NATIVE_BOOTSTRAP;
  return {
    locationHref: location.href,
    readyState: document.readyState,
    titleLength: document.title.length,
    syncMarkerPresent: Boolean(status),
    syncStatus: status?.getAttribute("data-deve-sync-status") ?? null,
    loginMarkerPresent: Boolean(document.querySelector("#login-username")),
    nativeBootstrap: {
      present: bootstrap != null,
      serviceState: bootstrap?.service_state ?? null,
      sessionBound: bootstrap?.session_bound === true,
      blockedReason: bootstrap?.blocked_reason ?? null,
    },
    body: {
      childElementCount: document.body?.childElementCount ?? 0,
      textLength: document.body?.textContent?.length ?? 0,
      htmlLength: document.body?.innerHTML.length ?? 0,
    },
    scriptCount: document.scripts.length,
  };
}

export function remoteEntrySurfacePresent(expectedOrigin) {
  try {
    return location.origin === new URL(expectedOrigin).origin
      && Boolean(document.querySelector("[data-deve-sync-status], #login-username"));
  } catch {
    return false;
  }
}

export async function reloadPageAndWaitForNewMainDocument(
  page,
  withDeadline,
  timeoutMs = 30_000,
) {
  await page.send("Page.enable", {}, timeoutMs);
  const frameTree = await page.send("Page.getFrameTree", {}, timeoutMs);
  const initialFrame = frameTree?.frameTree?.frame;
  if (typeof initialFrame?.id !== "string" || typeof initialFrame?.loaderId !== "string") {
    throw new Error("Android WebView main document identity unavailable before reload");
  }

  let removeListener = () => {};
  let cancelNextDocument = () => {};
  const nextDocument = new Promise((resolve) => {
    cancelNextDocument = () => resolve(null);
    removeListener = page.on("Page.frameNavigated", ({ frame }) => {
      if (frame?.id === initialFrame.id
        && !frame.parentId
        && typeof frame.loaderId === "string"
        && frame.loaderId !== initialFrame.loaderId) {
        resolve(frame);
      }
    });
  });
  let boundedNextDocument;
  try {
    boundedNextDocument = Promise.resolve().then(() => withDeadline(
      "Android WebView new main document",
      nextDocument,
      timeoutMs,
    ));
    const [, nextFrame] = await Promise.all([
      page.send("Page.reload", { ignoreCache: true, loaderId: initialFrame.loaderId }, timeoutMs),
      boundedNextDocument,
    ]);
    return nextFrame;
  } finally {
    removeListener();
    cancelNextDocument();
    await boundedNextDocument?.catch(() => {});
  }
}

export async function fetchCdpTargets(cdpEndpoint, withDeadline, timeoutMs) {
  const controller = new AbortController();
  try {
    return await withDeadline("Android WebView target discovery", (async () => {
      const response = await fetch(`${cdpEndpoint}/json`, { signal: controller.signal });
      if (!response.ok) throw new Error(`CDP target discovery returned ${response.status}`);
      return response.json();
    })(), timeoutMs);
  } finally {
    controller.abort();
  }
}

function allowlistedEnum(value, allowed) {
  if (value == null) return null;
  return allowed.has(value) ? value : "unknown";
}

function classifyLocation(value, expectedOrigin) {
  try {
    const url = new URL(value);
    if (expectedOrigin && url.origin === new URL(expectedOrigin).origin) return "expected-origin";
    if (!expectedOrigin && url.origin === "http://tauri.localhost") return "bundled-local";
    return "unexpected-origin";
  } catch {
    return "invalid";
  }
}

function sanitizePageSnapshot(snapshot, expectedOrigin, generation) {
  return {
    generation,
    locationClass: classifyLocation(snapshot?.locationHref, expectedOrigin),
    readyState: allowlistedEnum(snapshot?.readyState, DOCUMENT_READY_STATES),
    titleLength: Number.isSafeInteger(snapshot?.titleLength) ? snapshot.titleLength : null,
    syncMarkerPresent: snapshot?.syncMarkerPresent === true,
    syncStatus: allowlistedEnum(snapshot?.syncStatus, SYNC_STATUS_VALUES),
    loginMarkerPresent: snapshot?.loginMarkerPresent === true,
    nativeBootstrap: {
      present: snapshot?.nativeBootstrap?.present === true,
      serviceState: allowlistedEnum(
        snapshot?.nativeBootstrap?.serviceState, NATIVE_SERVICE_STATE_VALUES,
      ),
      sessionBound: snapshot?.nativeBootstrap?.sessionBound === true,
      blockedReason: allowlistedEnum(
        snapshot?.nativeBootstrap?.blockedReason, NATIVE_BLOCKED_REASON_VALUES,
      ),
    },
    body: {
      childElementCount: Number.isSafeInteger(snapshot?.body?.childElementCount)
        ? snapshot.body.childElementCount : null,
      textLength: Number.isSafeInteger(snapshot?.body?.textLength) ? snapshot.body.textLength : null,
      htmlLength: Number.isSafeInteger(snapshot?.body?.htmlLength) ? snapshot.body.htmlLength : null,
    },
    scriptCount: Number.isSafeInteger(snapshot?.scriptCount) ? snapshot.scriptCount : null,
  };
}

function snapshotMatchesOrigin(snapshot, expectedOrigin) {
  return snapshot.locationClass === (expectedOrigin ? "expected-origin" : "bundled-local");
}

function snapshotMatchesRequiredSurface(snapshot, requiredSurface) {
  if (requiredSurface === "sync") return snapshot.syncMarkerPresent;
  return snapshot.syncMarkerPresent || snapshot.loginMarkerPresent;
}

function retryableDiscoveryError(error) {
  const message = String(error?.message ?? error);
  return /CDP socket (closed|failed)|Inspected target navigated or closed|Android WebView target unavailable/i
    .test(message)
    || /^timeout during (Android WebView (CDP socket open|target discovery)|Runtime\.(enable|evaluate))$/i
      .test(message);
}

function safeDiscoveryFailure(error) {
  const message = String(error?.message ?? error);
  if (/CDP socket closed/i.test(message)) return "CDP socket closed";
  if (/CDP socket failed/i.test(message)) return "CDP socket failed";
  if (/Inspected target navigated or closed/i.test(message)) return "CDP target retired";
  if (/Android WebView target unavailable/i.test(message)) return "Android WebView target unavailable";
  const timeout = message.match(/^timeout during (Android WebView (?:CDP socket open|target discovery)|Runtime\.(?:enable|evaluate))$/i);
  if (timeout) return `timeout during ${timeout[1]}`;
  return "non-retryable Android WebView CDP discovery failure";
}

function targetMatchesOrigin(target, expectedOrigin) {
  if (!target.webSocketDebuggerUrl || target.type !== "page") return false;
  return classifyLocation(target.url, expectedOrigin)
    === (expectedOrigin ? "expected-origin" : "bundled-local");
}

function targetGenerationKey(target) {
  const id = typeof target.id === "string" ? target.id : "";
  return `${id}\0${target.webSocketDebuggerUrl}`;
}

function stableDeadlineError() {
  const error = new Error("stable Android WebView discovery deadline expired");
  error.code = "DEVE_ANDROID_CDP_STABLE_DEADLINE";
  return error;
}

export async function findStableAppPage({
  cdpEndpoint,
  withDeadline,
  expectedOrigin,
  requiredSurface = "sync",
  testing,
}) {
  if (!REQUIRED_PAGE_SURFACES.has(requiredSurface)) {
    throw new Error(`unsupported Android WebView required surface: ${requiredSurface}`);
  }
  const hooks = testing ?? {};
  const now = hooks.now ?? (() => performance.now());
  const sleep = hooks.sleep
    ?? ((milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)));
  const pollIntervalMs = hooks.pollIntervalMs ?? DISCOVERY_POLL_INTERVAL_MS;
  const generationTimeoutMs = hooks.generationTimeoutMs ?? PAGE_GENERATION_TIMEOUT_MS;
  const stableTimeoutMs = hooks.stableTimeoutMs ?? STABLE_PAGE_TIMEOUT_MS;
  const listTargets = async (timeoutMs) => {
    if (hooks.listTargets) return hooks.listTargets(timeoutMs);
    return fetchCdpTargets(cdpEndpoint, withDeadline, timeoutMs);
  };

  const deadline = now() + stableTimeoutMs;
  let page = null;
  let activeTargetKey = null;
  let generationStartedAt = null;
  let generation = 0;
  let lastPageSnapshot = null;
  let lastPageFailure = "Android WebView target unavailable";
  const retiredTargetKeys = new Set();

  const commandBudget = () => {
    const remaining = deadline - now();
    if (remaining <= 0) throw stableDeadlineError();
    return Math.max(1, Math.min(DISCOVERY_COMMAND_TIMEOUT_MS, remaining));
  };

  const completeWithinDeadline = async (operation, retireLateResult) => {
    const result = await operation(commandBudget());
    if (now() >= deadline) {
      await retireLateResult?.(result).catch(() => {});
      throw stableDeadlineError();
    }
    return result;
  };

  const retirePage = async ({ retireGeneration = false } = {}) => {
    const retired = page;
    page = null;
    if (retireGeneration && activeTargetKey) {
      retiredTargetKeys.add(activeTargetKey);
      activeTargetKey = null;
      generationStartedAt = null;
    }
    await retired?.close();
  };

  while (now() < deadline) {
    try {
      if (activeTargetKey && now() - generationStartedAt >= generationTimeoutMs) {
        lastPageFailure = `renderer generation lease expired after ${generationTimeoutMs}ms`;
        await retirePage({ retireGeneration: true });
      }

      if (!page) {
        const targets = await completeWithinDeadline((timeoutMs) => listTargets(timeoutMs));
        const target = targets.find((candidate) => targetMatchesOrigin(candidate, expectedOrigin)
          && !retiredTargetKeys.has(targetGenerationKey(candidate)));
        if (!target) {
          if (retiredTargetKeys.size === 0) {
            lastPageFailure = "Android WebView target unavailable";
          }
          await sleep(Math.min(pollIntervalMs, Math.max(1, deadline - now())));
          continue;
        }
        const targetKey = targetGenerationKey(target);
        if (targetKey !== activeTargetKey) {
          // Superseding a renderer is one-way: an older key cannot return later
          // and acquire a fresh lease through an A -> B -> A target sequence.
          if (activeTargetKey) retiredTargetKeys.add(activeTargetKey);
          activeTargetKey = targetKey;
          generationStartedAt = now();
          generation += 1;
        }
        console.log("mobile-android-lifecycle: attaching page CDP");
        page = await completeWithinDeadline(
          (timeoutMs) => (hooks.connectPage
            ? hooks.connectPage(target.webSocketDebuggerUrl, timeoutMs)
            : CdpPage.connect(target.webSocketDebuggerUrl, withDeadline, commandBudget)),
          (latePage) => latePage.close(),
        );
        console.log("mobile-android-lifecycle: page CDP attached");
      }

      const rawSnapshot = await completeWithinDeadline((timeoutMs) => page.callWithin(
        timeoutMs, readPageSnapshot,
      ));
      const snapshot = sanitizePageSnapshot(rawSnapshot, expectedOrigin, generation);
      lastPageSnapshot = snapshot;
      if (!snapshotMatchesOrigin(snapshot, expectedOrigin)) {
        lastPageFailure = "Android WebView target navigated away from the expected origin";
        // The target list and the attached Runtime snapshot are not atomic. Close
        // this connection, but keep the target-bound generation lease so the same
        // renderer can be rediscovered if a transient blank navigation settles.
        // Exact-origin and marker checks still run again before helpers are installed.
        await retirePage();
      } else if (now() - generationStartedAt >= generationTimeoutMs) {
        lastPageFailure = `renderer generation lease expired after ${generationTimeoutMs}ms`;
        await retirePage({ retireGeneration: true });
      } else if (snapshotMatchesRequiredSurface(snapshot, requiredSurface)) {
        await completeWithinDeadline((timeoutMs) => page.evaluate(
          `globalThis.__deveVisibleElement = ${visibleElement.toString()}`, timeoutMs,
        ));
        return page;
      } else {
        const marker = requiredSurface === "remote-entry" ? "remote entry marker" : "sync marker";
        lastPageFailure = `${marker} absent while document.readyState=${snapshot.readyState ?? "unknown"}`;
      }
    } catch (error) {
      if (error?.code === "DEVE_ANDROID_CDP_STABLE_DEADLINE") {
        lastPageFailure = "stable Android WebView discovery deadline expired";
        await retirePage();
        break;
      }
      lastPageFailure = safeDiscoveryFailure(error);
      await retirePage();
      if (!retryableDiscoveryError(error)) {
        throw new Error(
          `Android WebView CDP discovery failed; lastPageFailure=${lastPageFailure}; `
            + `page=${JSON.stringify(lastPageSnapshot)}`,
        );
      }
      console.log(`mobile-android-lifecycle: retrying page CDP: ${lastPageFailure}`);
    }

    await sleep(Math.min(pollIntervalMs, Math.max(1, deadline - now())));
  }

  await retirePage();
  throw new Error(
    `timeout waiting for stable Android WebView page; lastPageFailure=${lastPageFailure}; `
      + `page=${JSON.stringify(lastPageSnapshot)}`,
  );
}

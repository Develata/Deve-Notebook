import assert from "node:assert/strict";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export function isDirectInvocation(argvPath, moduleUrl) {
  return Boolean(argvPath) && moduleUrl === pathToFileURL(resolve(argvPath)).href;
}

export const renderedShellSelector = "#login-username, [data-deve-sync-status]";

export function renderedShellPresent(selector, root = document) {
  return root.querySelector(selector) != null;
}

export async function waitForRenderedShell(page, timeout = 15000) {
  await page.waitForFunction(renderedShellPresent, renderedShellSelector, { timeout });
}

export function editorContentIncludes(expected, root = window) {
  if (typeof root.getEditorContent !== "function") {
    return false;
  }
  const content = root.getEditorContent();
  return typeof content === "string" && content.includes(expected);
}

export function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function waitUntil(label, predicate, timeout, pollMs = 250) {
  const started = Date.now();
  let lastError;
  while (Date.now() - started < timeout) {
    try {
      if (await predicate()) {
        return;
      }
    } catch (err) {
      lastError = err;
    }
    await delay(pollMs);
  }
  const suffix = lastError ? `: ${lastError.message}` : "";
  throw new Error(`timeout waiting for ${label}${suffix}`);
}

export function webSocketMatchesExpectedOrigin(url, httpOrigin) {
  try {
    const expected = new URL(httpOrigin);
    expected.protocol = expected.protocol === "https:" ? "wss:" : "ws:";
    const observed = new URL(url);
    return observed.origin === expected.origin && observed.pathname === "/ws";
  } catch {
    return false;
  }
}

function isExpectedRuntimeRequest(url, httpOrigin) {
  try {
    const expected = new URL(httpOrigin);
    const observed = new URL(url);
    return webSocketMatchesExpectedOrigin(url, httpOrigin)
      || (observed.origin === expected.origin && observed.pathname.startsWith("/api/"));
  } catch {
    return false;
  }
}

export function isExpectedRestartTransportError(
  message,
  expectedOrigin,
  requestFailures = [],
) {
  const socketUrl = message.match(/WebSocket connection to ['"]?([^'"\s]+)['"]? failed/u)?.[1];
  if (socketUrl) {
    return webSocketMatchesExpectedOrigin(socketUrl, expectedOrigin);
  }
  const errorText = message.match(
    /net::(?:ERR_CONNECTION_(?:REFUSED|RESET)|ERR_SOCKET_NOT_CONNECTED)/u,
  )?.[0];
  return Boolean(errorText)
    && requestFailures.some((failure) =>
      failure.duringHostRestart
      && failure.errorText === errorText
      && isExpectedRuntimeRequest(failure.url, expectedOrigin));
}

export function attachDiagnostics(page, label, expectedOrigin) {
  const responseStatusByRequest = new WeakMap();
  const diag = {
    label,
    expectedOrigin,
    sockets: [],
    responses: [],
    requestFailures: [],
    consoleErrors: [],
    pageErrors: [],
    offline: false,
    hostRestart: false,
  };

  page.on("websocket", (ws) => {
    const socket = { url: ws.url(), frames: 0 };
    diag.sockets.push(socket);
    ws.on("framereceived", () => {
      socket.frames += 1;
    });
  });
  page.on("response", (response) => {
    responseStatusByRequest.set(response.request(), response.status());
    const url = new URL(response.url());
    if (url.pathname.startsWith("/api/")) {
      diag.responses.push({ path: url.pathname, status: response.status() });
    }
  });
  page.on("requestfailed", (request) => {
    diag.requestFailures.push({
      url: request.url(),
      errorText: request.failure()?.errorText ?? "unknown",
      duringOffline: diag.offline,
      duringHostRestart: diag.hostRestart,
      responseStatus: responseStatusByRequest.get(request),
    });
  });
  page.on("console", (msg) => {
    if (msg.type() === "error") {
      const message = msg.text();
      diag.consoleErrors.push({
        message,
        duringOffline: diag.offline,
        duringHostRestart: diag.hostRestart,
      });
      if (!diag.offline && !diag.hostRestart) {
        console.error(`docker-multiclient-smoke: ${label} console error: ${message}`);
      }
    }
  });
  page.on("pageerror", (err) => {
    const detail = err.stack || err.message;
    diag.pageErrors.push(detail);
    console.error(`docker-multiclient-smoke: ${label} page error: ${detail}`);
  });

  return diag;
}

export function relevantConsoleErrors(diag) {
  return diag.consoleErrors.filter(({ message, duringOffline, duringHostRestart }) => {
    if (message.includes("favicon.ico")) {
      return false;
    }
    if (duringOffline && message.includes("net::ERR_INTERNET_DISCONNECTED")) {
      return false;
    }
    return !(
      duringHostRestart
      && isExpectedRestartTransportError(
        message,
        diag.expectedOrigin,
        diag.requestFailures,
      )
    );
  });
}

export function relevantRequestFailures(diag) {
  return diag.requestFailures.filter(({
    url,
    errorText,
    duringOffline,
    duringHostRestart,
    responseStatus,
  }) => {
    if (
      responseStatus === 204
      && errorText === "net::ERR_ABORTED"
      && isExpectedRuntimeRequest(url, diag.expectedOrigin)
    ) {
      return false;
    }
    if (
      duringOffline
      && isExpectedRuntimeRequest(url, diag.expectedOrigin)
      && errorText === "net::ERR_INTERNET_DISCONNECTED"
    ) {
      return false;
    }
    if (
      duringHostRestart
      && isExpectedRuntimeRequest(url, diag.expectedOrigin)
      && /net::(?:ERR_CONNECTION_(?:REFUSED|RESET)|ERR_SOCKET_NOT_CONNECTED)/u.test(errorText)
    ) {
      return false;
    }
    return true;
  });
}

export async function readNodeRole(baseUrl, timeoutMs = 2000) {
  const response = await fetch(`${baseUrl}/api/node/role`, {
    signal: AbortSignal.timeout(timeoutMs),
  });
  assert.equal(response.status, 200);
  const role = await response.json();
  assert.match(role.host_peer_id, /^[a-z0-9]+$/u);
  assert.match(
    role.runtime_incarnation,
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u,
  );
  return role;
}

export async function waitForRestartedNodeRole({
  baseUrl,
  before,
  timeoutMs,
}) {
  let restarted;
  await waitUntil("candidate server restart", async () => {
    try {
      const observed = await readNodeRole(baseUrl);
      if (
        observed.host_peer_id === before.host_peer_id
        && observed.runtime_incarnation !== before.runtime_incarnation
      ) {
        restarted = observed;
        return true;
      }
      return false;
    } catch {
      return false;
    }
  }, timeoutMs);
  return restarted;
}

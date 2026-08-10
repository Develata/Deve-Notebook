import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import test from "node:test";
import {
  attachDiagnostics,
  beginHostRestart,
  closeBrowserResources,
  endHostRestart,
  relevantConsoleErrors,
  relevantRequestFailures,
} from "./lib/docker-multiclient-runtime.mjs";

const expectedOrigin = "http://127.0.0.1:3101";
const resourceMessage = "Failed to load resource: net::ERR_CONNECTION_REFUSED";

test("request restart generation is captured when the request starts", async () => {
  const page = new EventEmitter();
  const diag = attachDiagnostics(page, "generation-test", expectedOrigin);
  const request = {
    url: () => `${expectedOrigin}/api/node/role`,
    method: () => "GET",
    failure: () => ({ errorText: "net::ERR_CONNECTION_REFUSED" }),
  };

  beginHostRestart([diag]);
  page.emit("request", request);
  await endHostRestart([diag]);
  page.emit("requestfailed", request);

  assert.equal(diag.requestFailures.length, 1);
  assert.equal(diag.requestFailures[0].restartGeneration, 1);
  assert.equal(diag.requestFailures[0].duringHostRestart, true);
  assert.deepEqual(relevantRequestFailures(diag), []);
  await assert.rejects(endHostRestart([diag]), /restart window is not active/);
});

test("restart generation rejects nested windows and increments exactly once", async () => {
  const diag = {
    label: "window-test",
    hostRestart: false,
    restartGeneration: 0,
  };
  beginHostRestart([diag]);
  assert.equal(diag.restartGeneration, 1);
  assert.throws(() => beginHostRestart([diag]), /restart window is already active/);
  await endHostRestart([diag]);
  assert.equal(diag.hostRestart, false);
});

test("restart transitions validate every diagnostic before mutating any", async () => {
  const valid = { label: "valid", hostRestart: false, restartGeneration: 0 };
  const alreadyActive = { label: "active", hostRestart: true, restartGeneration: 3 };
  assert.throws(
    () => beginHostRestart([valid, alreadyActive]),
    /restart window is already active/,
  );
  assert.deepEqual(valid, { label: "valid", hostRestart: false, restartGeneration: 0 });

  const firstActive = { label: "first", hostRestart: true, restartGeneration: 1 };
  const inactive = { label: "inactive", hostRestart: false, restartGeneration: 2 };
  await assert.rejects(
    endHostRestart([firstActive, inactive]),
    /restart window is not active/,
  );
  assert.equal(firstActive.hostRestart, true);

  const secondActive = { label: "second", hostRestart: true, restartGeneration: 1 };
  const ending = endHostRestart([firstActive, secondActive]);
  setTimeout(() => {
    secondActive.hostRestart = false;
  }, 0);
  await assert.rejects(ending, /restart window ended during drain/);
  assert.equal(firstActive.hostRestart, true);
});

test("restart end drains same-origin Chromium resource errors into the active generation", async () => {
  const page = new EventEmitter();
  const diag = attachDiagnostics(page, "drain-test", expectedOrigin);
  beginHostRestart([diag]);

  const ending = endHostRestart([diag]);
  setTimeout(() => {
    page.emit("console", {
      type: () => "error",
      text: () => "Failed to load resource: net::ERR_SOCKET_NOT_CONNECTED",
      location: () => ({ url: `${expectedOrigin}/api/node/role` }),
    });
  }, 0);
  await ending;

  assert.equal(diag.hostRestart, false);
  assert.equal(diag.consoleErrors[0].restartGeneration, 1);
  assert.deepEqual(relevantConsoleErrors(diag), []);

  page.emit("console", {
    type: () => "error",
    text: () => "Failed to load resource: net::ERR_SOCKET_NOT_CONNECTED",
    location: () => ({ url: `${expectedOrigin}/api/node/role` }),
  });
  assert.equal(diag.consoleErrors[1].restartGeneration, null);
  assert.deepEqual(relevantConsoleErrors(diag), [diag.consoleErrors[1]]);
});

test("delayed resource console correlation is bounded and one-to-one", () => {
  const restartFailure = {
    url: `${expectedOrigin}/api/node/role`,
    errorText: "net::ERR_CONNECTION_REFUSED",
    duringHostRestart: true,
    restartGeneration: 1,
    responseStatus: undefined,
    observedAtMs: 1000,
  };
  const correlated = {
    message: resourceMessage,
    locationUrl: "",
    duringOffline: false,
    duringHostRestart: false,
    restartGeneration: null,
    observedAtMs: 1001,
  };
  const unpaired = { ...correlated, observedAtMs: 1002 };
  const outsideSocket = {
    ...correlated,
    message: "WebSocket connection to 'ws://127.0.0.1:3101/ws' failed",
    observedAtMs: 1003,
  };
  const late = { ...correlated, observedAtMs: 6001 };

  assert.deepEqual(
    relevantConsoleErrors({
      expectedOrigin,
      requestFailures: [restartFailure],
      consoleErrors: [correlated, unpaired, outsideSocket, late],
    }),
    [unpaired, outsideSocket, late],
  );
});

test("active console errors require a request failure from the same restart generation", () => {
  const historicalFailure = {
    url: `${expectedOrigin}/api/node/role`,
    method: "GET",
    errorText: "net::ERR_CONNECTION_REFUSED",
    duringHostRestart: true,
    restartGeneration: 1,
    responseStatus: undefined,
    observedAtMs: 1000,
  };
  const currentConsole = {
    message: resourceMessage,
    locationUrl: "",
    duringOffline: false,
    duringHostRestart: true,
    restartGeneration: 2,
    observedAtMs: 1001,
  };

  assert.deepEqual(
    relevantConsoleErrors({
      expectedOrigin,
      requestFailures: [historicalFailure],
      consoleErrors: [currentConsole],
    }),
    [currentConsole],
  );
  assert.deepEqual(
    relevantConsoleErrors({
      expectedOrigin,
      requestFailures: [{ ...historicalFailure, restartGeneration: 2 }],
      consoleErrors: [currentConsole],
    }),
    [],
  );
});

test("restart resource errors require both active generation and expected origin", () => {
  const duringRestart = {
    message: resourceMessage,
    locationUrl: `${expectedOrigin}/api/node/role`,
    duringOffline: false,
    duringHostRestart: true,
    restartGeneration: 1,
    observedAtMs: 1000,
  };
  const postWindow = {
    ...duringRestart,
    duringHostRestart: false,
    restartGeneration: null,
  };
  const outsideOrigin = {
    ...duringRestart,
    locationUrl: "https://example.invalid/resource",
  };
  const missingOrigin = {
    ...duringRestart,
    locationUrl: "",
  };

  assert.deepEqual(
    relevantConsoleErrors({
      expectedOrigin,
      requestFailures: [],
      consoleErrors: [duringRestart, postWindow, outsideOrigin, missingOrigin],
    }),
    [postWindow, outsideOrigin, missingOrigin],
  );
});

test("network-changed resources are accepted only inside the exact restart generation", () => {
  const scopedConsole = {
    message: "Failed to load resource: net::ERR_NETWORK_CHANGED",
    locationUrl: `${expectedOrigin}/assets/editor.css`,
    duringOffline: false,
    duringHostRestart: true,
    restartGeneration: 1,
    observedAtMs: 1000,
  };
  const postRestartConsole = {
    ...scopedConsole,
    duringHostRestart: false,
    restartGeneration: null,
  };
  const scopedRequest = {
    url: `${expectedOrigin}/assets/editor.css`,
    method: "GET",
    errorText: "net::ERR_NETWORK_CHANGED",
    duringOffline: false,
    duringHostRestart: true,
    restartGeneration: 1,
    responseStatus: undefined,
  };
  const foreignRequest = {
    ...scopedRequest,
    url: "https://example.invalid/assets/editor.css",
  };
  const mutationRequest = {
    ...scopedRequest,
    url: `${expectedOrigin}/api/repos/remove`,
    method: "POST",
  };
  const completedRequest = {
    ...scopedRequest,
    responseStatus: 200,
  };

  assert.deepEqual(
    relevantConsoleErrors({
      expectedOrigin,
      requestFailures: [],
      consoleErrors: [scopedConsole, postRestartConsole],
    }),
    [postRestartConsole],
  );
  assert.deepEqual(
    relevantRequestFailures({
      expectedOrigin,
      requestFailures: [scopedRequest, foreignRequest, mutationRequest, completedRequest],
    }),
    [foreignRequest, mutationRequest, completedRequest],
  );
});

test("post-window request failures remain fail-closed even with old restart history", () => {
  const restartFailure = {
    url: `${expectedOrigin}/api/node/role`,
    errorText: "net::ERR_CONNECTION_RESET",
    duringHostRestart: true,
    restartGeneration: 1,
    responseStatus: undefined,
    observedAtMs: 1000,
  };
  const postWindowFailure = {
    ...restartFailure,
    duringHostRestart: false,
    restartGeneration: null,
    observedAtMs: 1001,
  };

  assert.deepEqual(
    relevantRequestFailures({
      expectedOrigin,
      requestFailures: [restartFailure, postWindowFailure],
    }),
    [postWindowFailure],
  );
});

test("browser cleanup attempts every resource and fails on any cleanup error", async () => {
  const closed = [];
  const contexts = [
    { close: async () => closed.push("first") },
    {
      close: async () => {
        closed.push("second");
        throw new Error("context cleanup failed");
      },
    },
  ];
  const browser = {
    close: async () => {
      closed.push("browser");
      throw new Error("browser cleanup failed");
    },
  };

  await assert.rejects(
    closeBrowserResources(contexts, browser),
    (error) => error instanceof AggregateError && error.errors.length === 2,
  );
  assert.deepEqual(closed, ["second", "first", "browser"]);

  await closeBrowserResources([{ close: async () => {} }], { close: async () => {} });
});

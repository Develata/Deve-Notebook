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

test("request restart generation is captured when the request starts", () => {
  const page = new EventEmitter();
  const diag = attachDiagnostics(page, "generation-test", expectedOrigin);
  const request = {
    url: () => `${expectedOrigin}/api/node/role`,
    failure: () => ({ errorText: "net::ERR_CONNECTION_REFUSED" }),
  };

  beginHostRestart([diag]);
  page.emit("request", request);
  endHostRestart([diag]);
  page.emit("requestfailed", request);

  assert.equal(diag.requestFailures.length, 1);
  assert.equal(diag.requestFailures[0].restartGeneration, 1);
  assert.equal(diag.requestFailures[0].duringHostRestart, true);
  assert.deepEqual(relevantRequestFailures(diag), []);
  assert.throws(() => endHostRestart([diag]), /restart window is not active/);
});

test("restart generation rejects nested windows and increments exactly once", () => {
  const diag = {
    label: "window-test",
    hostRestart: false,
    restartGeneration: 0,
  };
  beginHostRestart([diag]);
  assert.equal(diag.restartGeneration, 1);
  assert.throws(() => beginHostRestart([diag]), /restart window is already active/);
  endHostRestart([diag]);
  assert.equal(diag.hostRestart, false);
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

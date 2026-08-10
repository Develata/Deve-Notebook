import assert from "node:assert/strict";
import test from "node:test";

import {
  CdpPage,
  findStableAppPage,
  isExpectedCdpTargetRetirement,
} from "./lib/android-webview-cdp.mjs";

class SilentSocket extends EventTarget {
  static CLOSED = 3;

  constructor() {
    super();
    this.readyState = 0;
    queueMicrotask(() => {
      this.readyState = 1;
      this.dispatchEvent(new Event("open"));
    });
  }

  send() {}

  close() {
    this.readyState = SilentSocket.CLOSED;
    this.dispatchEvent(new Event("close"));
  }
}

function fakeClock() {
  let current = 0;
  return {
    now: () => current,
    advance: (milliseconds) => {
      current += milliseconds;
    },
    sleep: async (milliseconds) => {
      current += milliseconds;
    },
  };
}

function pageSnapshot({
  marker = false,
  loginMarker = false,
  locationHref = "http://tauri.localhost/?token=secret-value#private",
  injectShortSecrets = false,
} = {}) {
  return {
    locationHref,
    readyState: "complete",
    titleLength: 9,
    title: "private note title",
    syncMarkerPresent: marker,
    syncStatus: injectShortSecrets ? "short-session" : marker ? "handshaking-repo" : null,
    loginMarkerPresent: loginMarker,
    nativeBootstrap: {
      present: true,
      serviceState: injectShortSecrets ? "tiny-token" : "endpoint_session_ready",
      sessionBound: true,
      blockedReason: injectShortSecrets ? "short-secret" : null,
      httpBase: "http://127.0.0.1:43123",
    },
    body: {
      childElementCount: 1,
      textLength: 48,
      htmlLength: 96,
      preview: "Password with spaces and short-token must never be logged",
    },
    scriptCount: 4,
    inputValue: "do-not-log",
  };
}

function mockPage(observations) {
  let lastObservation = observations.at(-1);
  return {
    closed: 0,
    visibleHelperInstalled: 0,
    async callWithin() {
      const observation = observations.length > 0 ? observations.shift() : lastObservation;
      lastObservation = observation;
      if (observation instanceof Error) throw observation;
      if (typeof observation === "function") return observation();
      return observation;
    },
    async evaluate() {
      this.visibleHelperInstalled += 1;
    },
    async close() {
      this.closed += 1;
    },
  };
}

function discoveryArgs(testing) {
  return {
    cdpEndpoint: "http://android.test",
    withDeadline: async (_label, promise) => promise,
    waitUntil: async (_label, predicate) => predicate(),
    testing,
  };
}

function target(id = "page", url = "http://tauri.localhost/") {
  return [{
    id,
    type: "page",
    title: "private target title",
    url,
    webSocketDebuggerUrl: `ws://android.test/${id}`,
  }];
}

test("Android CDP discovery bounds Runtime.enable and retires its waiter", async () => {
  const originalWebSocket = globalThis.WebSocket;
  const observed = [];
  globalThis.WebSocket = SilentSocket;
  const withDeadline = async (label, promise, limit) => {
    observed.push({ label, limit });
    if (label === "Runtime.enable") throw new Error("synthetic command timeout");
    return promise;
  };

  try {
    await assert.rejects(
      CdpPage.connect("ws://android.test/page", withDeadline),
      /synthetic command timeout/,
    );
  } finally {
    if (originalWebSocket === undefined) delete globalThis.WebSocket;
    else globalThis.WebSocket = originalWebSocket;
  }

  assert.deepEqual(
    observed.find(({ label }) => label === "Runtime.enable"),
    { label: "Runtime.enable", limit: 10_000 },
  );
});

test("timed-out Android CDP commands cannot retain pending response waiters", async () => {
  const socket = new SilentSocket();
  const page = new CdpPage(socket, async (label, _promise, limit) => {
    assert.equal(label, "Runtime.evaluate");
    assert.equal(limit, 25);
    throw new Error("synthetic command timeout");
  });

  await assert.rejects(
    page.send("Runtime.evaluate", {}, 25),
    /synthetic command timeout/,
  );
  assert.equal(page.pending.size, 0);
});

test("graceful exit accepts only CDP target retirement transport races", () => {
  assert.equal(
    isExpectedCdpTargetRetirement(new Error("CDP socket closed during Runtime.evaluate")),
    true,
  );
  assert.equal(isExpectedCdpTargetRetirement("Inspected target navigated or closed"), true);
  assert.equal(isExpectedCdpTargetRetirement(new Error("native command denied")), false);
});

test("stable discovery keeps one healthy slow page generation attached", async () => {
  const clock = fakeClock();
  const page = mockPage([
    pageSnapshot(),
    pageSnapshot(),
    pageSnapshot({ marker: true }),
  ]);
  let connections = 0;

  const stable = await findStableAppPage(discoveryArgs({
    ...clock,
    listTargets: async () => target(),
    connectPage: async () => {
      connections += 1;
      return page;
    },
  }));

  assert.equal(stable, page);
  assert.equal(connections, 1);
  assert.equal(page.closed, 0);
  assert.equal(page.visibleHelperInstalled, 1);
});

test("bundled-local discovery skips stale remote and recovery-anchor targets", async () => {
  const localPage = mockPage([pageSnapshot({ marker: true })]);
  const stable = await findStableAppPage(discoveryArgs({
    ...fakeClock(),
    listTargets: async () => [
      ...target("stale-remote", "https://remote.test/"), ...target("recovery-anchor", "about:blank"),
      ...target("fresh-local", "http://tauri.localhost/"),
    ],
    connectPage: async (webSocketDebuggerUrl) => {
      assert.equal(webSocketDebuggerUrl, "ws://android.test/fresh-local");
      return localPage;
    },
    pollIntervalMs: 100,
    generationTimeoutMs: 500,
    stableTimeoutMs: 1_000,
  }));
  assert.equal(stable, localPage);
});

test("login page admission requires the explicit RemoteBrowser entry surface", async () => {
  await assert.rejects(
    findStableAppPage({ ...discoveryArgs({}), requiredSurface: "login-only" }),
    /unsupported Android WebView required surface/,
  );

  const rejectedClock = fakeClock();
  const rejected = mockPage([pageSnapshot({
    loginMarker: true,
    locationHref: "https://remote.test/",
  })]);
  await assert.rejects(
    findStableAppPage({
      ...discoveryArgs({
        ...rejectedClock,
        listTargets: async () => target("remote-login", "https://remote.test/"),
        connectPage: async () => rejected,
        pollIntervalMs: 100,
        generationTimeoutMs: 200,
        stableTimeoutMs: 500,
      }),
      expectedOrigin: "https://remote.test",
    }),
    /renderer generation lease expired/,
  );

  const acceptedClock = fakeClock();
  const accepted = mockPage([pageSnapshot({
    loginMarker: true,
    locationHref: "https://remote.test/",
  })]);
  const stable = await findStableAppPage({
    ...discoveryArgs({
      ...acceptedClock,
      listTargets: async () => target("remote-login", "https://remote.test/"),
      connectPage: async () => accepted,
      pollIntervalMs: 100,
      generationTimeoutMs: 200,
      stableTimeoutMs: 500,
    }),
    expectedOrigin: "https://remote.test",
    requiredSurface: "remote-entry",
  });
  assert.equal(stable, accepted);
  assert.equal(accepted.visibleHelperInstalled, 1);
});

test("stable discovery retires a closed target and attaches its replacement", async () => {
  const clock = fakeClock();
  const first = mockPage([new Error("CDP socket closed during Runtime.evaluate")]);
  const second = mockPage([pageSnapshot({ marker: true })]);
  const pages = [first, second];
  let connections = 0;

  const stable = await findStableAppPage(discoveryArgs({
    ...clock,
    listTargets: async () => target(),
    connectPage: async () => {
      connections += 1;
      return pages.shift();
    },
    pollIntervalMs: 100,
    generationTimeoutMs: 500,
    stableTimeoutMs: 1_000,
  }));

  assert.equal(stable, second);
  assert.equal(connections, 2);
  assert.equal(first.closed, 1);
});

test("stable discovery retires a target that navigates away before accepting its replacement", async () => {
  const clock = fakeClock();
  const first = mockPage([pageSnapshot({
    marker: true,
    locationHref: "https://private.invalid/short-secret?token=tiny",
  })]);
  const second = mockPage([pageSnapshot({ marker: true })]);
  const pages = [first, second];
  let connections = 0;

  const stable = await findStableAppPage(discoveryArgs({
    ...clock,
    listTargets: async () => target(`page-${connections + 1}`),
    connectPage: async () => {
      connections += 1;
      return pages.shift();
    },
    pollIntervalMs: 100,
    generationTimeoutMs: 500,
    stableTimeoutMs: 1_000,
  }));

  assert.equal(stable, second);
  assert.equal(connections, 2);
  assert.equal(first.closed, 1);
  assert.equal(first.visibleHelperInstalled, 0);
});

test("stable discovery reconnects a transiently blank snapshot for the same target", async () => {
  const clock = fakeClock();
  const first = mockPage([pageSnapshot({
    marker: false,
    locationHref: "about:blank",
  })]);
  const second = mockPage([pageSnapshot({ marker: true })]);
  const pages = [first, second];
  let connections = 0;

  const stable = await findStableAppPage(discoveryArgs({
    ...clock,
    listTargets: async () => target("same-renderer"),
    connectPage: async () => {
      connections += 1;
      return pages.shift();
    },
    pollIntervalMs: 100,
    generationTimeoutMs: 500,
    stableTimeoutMs: 1_000,
  }));

  assert.equal(stable, second);
  assert.equal(connections, 2);
  assert.equal(first.closed, 1);
  assert.equal(first.visibleHelperInstalled, 0);
  assert.equal(second.visibleHelperInstalled, 1);
});

test("renderer generation lease cannot be renewed by reconnecting the same target", async () => {
  const clock = fakeClock();
  const pages = [];
  let connections = 0;

  await assert.rejects(
    findStableAppPage(discoveryArgs({
      ...clock,
      listTargets: async () => target("same-renderer"),
      connectPage: async () => {
        connections += 1;
        const page = mockPage([pageSnapshot({
          marker: false,
          locationHref: "about:blank",
        })]);
        pages.push(page);
        return page;
      },
      pollIntervalMs: 100,
      generationTimeoutMs: 200,
      stableTimeoutMs: 750,
    })),
    /renderer generation lease expired/,
  );

  assert.equal(connections, 2);
  assert.equal(pages.every(({ closed }) => closed === 1), true);
  assert.equal(pages.every(({ visibleHelperInstalled }) => visibleHelperInstalled === 0), true);
});

test("interleaved targets cannot renew a previously observed renderer generation", async () => {
  const clock = fakeClock();
  const firstA = mockPage([pageSnapshot({ marker: false, locationHref: "about:blank" })]);
  const firstB = mockPage([pageSnapshot({ marker: false, locationHref: "about:blank" })]);
  const renewedA = mockPage([pageSnapshot({ marker: true })]);
  const pages = [firstA, firstB, renewedA];
  const targetIds = ["renderer-a", "renderer-b", "renderer-a"];
  let connections = 0;

  await assert.rejects(
    findStableAppPage(discoveryArgs({
      ...clock,
      listTargets: async () => target(targetIds.shift() ?? "renderer-a"),
      connectPage: async () => {
        connections += 1;
        return pages.shift();
      },
      pollIntervalMs: 100,
      generationTimeoutMs: 150,
      stableTimeoutMs: 500,
    })),
    /renderer generation lease expired/,
  );

  assert.equal(connections, 2);
  assert.equal(firstA.closed, 1);
  assert.equal(firstB.closed, 1);
  assert.equal(renewedA.visibleHelperInstalled, 0);
});

test("stable discovery rejects a marker that arrives after its absolute deadline", async () => {
  const clock = fakeClock();
  const page = mockPage([() => {
    clock.advance(150);
    return pageSnapshot({ marker: true });
  }]);

  await assert.rejects(
    findStableAppPage(discoveryArgs({
      ...clock,
      listTargets: async () => target(),
      connectPage: async () => page,
      pollIntervalMs: 10,
      generationTimeoutMs: 500,
      stableTimeoutMs: 100,
    })),
    /stable Android WebView discovery deadline expired/,
  );

  assert.equal(page.closed, 1);
  assert.equal(page.visibleHelperInstalled, 0);
});

test("stable discovery rejects helper installation that completes after its absolute deadline", async () => {
  const clock = fakeClock();
  const page = mockPage([pageSnapshot({ marker: true })]);
  page.evaluate = async function evaluateAfterDeadline() {
    this.visibleHelperInstalled += 1;
    clock.advance(101);
  };

  await assert.rejects(
    findStableAppPage(discoveryArgs({
      ...clock,
      listTargets: async () => target(),
      connectPage: async () => page,
      pollIntervalMs: 10,
      generationTimeoutMs: 500,
      stableTimeoutMs: 100,
    })),
    /stable Android WebView discovery deadline expired/,
  );

  assert.equal(page.closed, 1);
  assert.equal(page.visibleHelperInstalled, 1);
});

test("stable discovery closes a page connection that completes after its absolute deadline", async () => {
  const clock = fakeClock();
  const page = mockPage([pageSnapshot({ marker: true })]);

  await assert.rejects(
    findStableAppPage(discoveryArgs({
      ...clock,
      listTargets: async () => target(),
      connectPage: async () => {
        clock.advance(101);
        return page;
      },
      pollIntervalMs: 10,
      generationTimeoutMs: 500,
      stableTimeoutMs: 100,
    })),
    /stable Android WebView discovery deadline expired/,
  );

  assert.equal(page.closed, 1);
  assert.equal(page.visibleHelperInstalled, 0);
});

test("stable discovery failure preserves only the latest sanitized page snapshot", async () => {
  const clock = fakeClock();
  const pages = [];

  await assert.rejects(
    findStableAppPage(discoveryArgs({
      ...clock,
      listTargets: async () => target(`generation-${pages.length + 1}`),
      connectPage: async () => {
        const page = mockPage([pageSnapshot({ injectShortSecrets: true })]);
        pages.push(page);
        return page;
      },
      pollIntervalMs: 100,
      generationTimeoutMs: 200,
      stableTimeoutMs: 750,
    })),
    (error) => {
      assert.match(error.message, /lastPageFailure=/);
      assert.match(error.message, new RegExp(`"generation":${pages.length}`));
      assert.match(error.message, /"locationClass":"bundled-local"/);
      assert.match(error.message, /"syncStatus":"unknown"/);
      assert.match(error.message, /"serviceState":"unknown"/);
      assert.match(error.message, /"blockedReason":"unknown"/);
      assert.doesNotMatch(
        error.message,
        /secret-value|do-not-log|43123|private\.invalid|private note title|short-token|short-session|tiny-token|short-secret/,
      );
      return true;
    },
  );

  assert.ok(pages.length >= 2);
  assert.equal(pages.every(({ closed }) => closed === 1), true);
});

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  fetchCdpTargets,
  reloadPageAndWaitForNewMainDocument,
  remoteEntrySurfacePresent,
} from "./lib/android-webview-cdp.mjs";
import {
  fillRemoteLoginCredentials,
  loginAndroidRemote,
  readRemoteEntryState,
  readRemoteReadyState,
  submitRemoteLogin,
} from "./lib/android-business-flow.mjs";

const browserSource = readFileSync(
  new URL("./smoke-mobile-android-remote-browser.mjs", import.meta.url),
  "utf8",
);
const hostSource = readFileSync(
  new URL("./smoke-mobile-android-remote-browser.sh", import.meta.url),
  "utf8",
);

test("Android RemoteBrowser smoke proves business flow, zero IPC, and native local recovery", () => {
  assert.match(browserSource, /loginAndroidRemote/);
  assert.match(browserSource, /createAndroidDocument/);
  assert.match(browserSource, /commitAndroidChange/);
  assert.match(browserSource, /exerciseAndroidLastRepoRemoval/);
  assert.match(browserSource, /repoRemovalNoScope/);
  assert.match(browserSource, /ipc\.localhost/);
  assert.match(browserSource, /__DEVE_NATIVE_BACKEND_CONFIG__/);
  assert.match(browserSource, /probeWebCryptoEd25519/);
  assert.match(browserSource, /smoke-mobile-android-remote-browser/);
  assert.match(browserSource, /uiautomator/);
  assert.match(browserSource, /Use Local Backend/);
  assert.match(browserSource, /native_backend_get_service_state/);
  assert.match(browserSource, /native_backend_get_recovery_state/);
  assert.match(browserSource, /RemoteBrowser CDP target retirement/);
  assert.match(browserSource, /processExitedAfterGracefulShutdown/);
  assert.match(browserSource, /authorityTupleChanged/);
  assert.match(browserSource, /freshLocalEndpointSessionScope/);
  assert.match(browserSource, /remoteAuthorityNotReused/);
  assert.match(browserSource, /appPidStable/);
  assert.match(browserSource, /requiredSurface:\s*"remote-entry"/);
  assert.match(browserSource, /page\.call\(remoteEntrySurfacePresent, remoteOrigin\)/);
  assert.match(browserSource, /reloadPageAndWaitForNewMainDocument/);
  assert.match(browserSource, /typeAndroidEditorText/);
  assert.match(
    browserSource,
    /inputEditorText:\s*inputAndroidEditorText/,
    "RemoteBrowser must establish the same native editor input connection as LocalBackend",
  );
});

test("Android RemoteBrowser host smoke is preference-driven and target-qualified", () => {
  assert.match(hostSource, /native-backend\.json/);
  assert.match(hostSource, /inspect-android-target-capability\.mjs/);
  assert.match(hostSource, /run-as/);
  // The injection must be ONE quoted remote command: adb shell flattens
  // multiple arguments without re-quoting, which runs the pipe/redirect in
  // the device outer shell instead of inside run-as.
  assert.match(
    hostSource,
    /shell "run-as \$APP_ID sh -c 'echo \$PREFERENCE_BASE64 \| base64 -d > native-backend\.json'"/,
  );
  assert.match(hostSource, /recovered to fresh LocalBackend runtime/);
  assert.doesNotMatch(hostSource, /--remote-url/);
});

test("RemoteBrowser entry admission binds exact origin and an actionable surface", () => {
  const originalLocation = Object.getOwnPropertyDescriptor(globalThis, "location");
  const originalDocument = Object.getOwnPropertyDescriptor(globalThis, "document");
  const originalVisible = globalThis.__deveVisibleElement;
  const state = { origin: "https://remote.test", sync: null, visible: new Set() };
  Object.defineProperty(globalThis, "location", {
    configurable: true,
    get: () => ({ origin: state.origin }),
  });
  Object.defineProperty(globalThis, "document", {
    configurable: true,
    value: {
      querySelector: (selector) => {
        if (selector === "[data-deve-sync-status], #login-username") {
          return state.sync || state.visible.has("#login-username") ? {} : null;
        }
        if (selector === "[data-deve-sync-status]") {
          return state.sync == null
            ? null
            : { getAttribute: () => state.sync };
        }
        return null;
      },
    },
  });
  globalThis.__deveVisibleElement = (selector) => state.visible.has(selector)
    ? { disabled: false }
    : null;

  try {
    state.visible = new Set(["#login-username"]);
    assert.equal(remoteEntrySurfacePresent("https://remote.test/path"), true);
    assert.equal(readRemoteEntryState("https://remote.test"), null);

    state.visible = new Set([
      "#login-username",
      "#login-password",
      'button[type="submit"]',
    ]);
    assert.deepEqual(readRemoteEntryState("https://remote.test"), { kind: "login" });

    state.visible = new Set();
    state.sync = "ready";
    assert.equal(remoteEntrySurfacePresent("https://remote.test"), true);
    assert.deepEqual(readRemoteEntryState("https://remote.test"), { kind: "ready" });
    assert.deepEqual(readRemoteReadyState("https://remote.test"), { kind: "ready" });

    state.origin = "https://redirected.test";
    state.visible = new Set(["#login-username"]);
    assert.equal(remoteEntrySurfacePresent("https://remote.test"), false);
    assert.deepEqual(
      readRemoteEntryState("https://remote.test"),
      { kind: "unexpected-origin" },
    );
    assert.deepEqual(
      readRemoteReadyState("https://remote.test"),
      { kind: "unexpected-origin" },
    );
    assert.equal(remoteEntrySurfacePresent("not an origin"), false);
  } finally {
    if (originalLocation) Object.defineProperty(globalThis, "location", originalLocation);
    else delete globalThis.location;
    if (originalDocument) Object.defineProperty(globalThis, "document", originalDocument);
    else delete globalThis.document;
    if (originalVisible === undefined) delete globalThis.__deveVisibleElement;
    else globalThis.__deveVisibleElement = originalVisible;
  }
});

test("RemoteBrowser reload waits for a new main-document loader", async () => {
  const sent = [];
  let listener = null;
  let removed = 0;
  const page = {
    on(method, callback) {
      assert.equal(method, "Page.frameNavigated");
      listener = callback;
      return () => {
        listener = null;
        removed += 1;
      };
    },
    async send(method, params) {
      sent.push({ method, params });
      if (method === "Page.getFrameTree") {
        return { frameTree: { frame: { id: "main", loaderId: "old" } } };
      }
      if (method === "Page.reload") {
        listener({ frame: { id: "main", loaderId: "old" } });
        listener({ frame: { id: "child", parentId: "main", loaderId: "child" } });
        listener({ frame: { id: "main", loaderId: "new" } });
      }
      return {};
    },
  };

  const frame = await reloadPageAndWaitForNewMainDocument(
    page,
    async (_label, promise) => promise,
    1234,
  );
  assert.equal(frame.loaderId, "new");
  assert.equal(removed, 1);
  assert.deepEqual(sent.at(-1), {
    method: "Page.reload",
    params: { ignoreCache: true, loaderId: "old" },
  });
});

test("RemoteBrowser reload cancellation retires its new-document waiter", async () => {
  for (const failure of ["reload", "deadline"]) {
    let listener = null;
    let removed = 0;
    let deadlineSettled = false;
    const page = {
      on(_method, callback) {
        listener = callback;
        return () => {
          listener = null;
          removed += 1;
        };
      },
      async send(method) {
        if (method === "Page.getFrameTree") {
          return { frameTree: { frame: { id: "main", loaderId: "old" } } };
        }
        if (method === "Page.reload" && failure === "reload") {
          throw new Error("synthetic reload failure");
        }
        return {};
      },
    };
    const withDeadline = async (_label, promise) => {
      try {
        if (failure === "deadline") throw new Error("synthetic document deadline");
        return await promise;
      } finally {
        deadlineSettled = true;
      }
    };
    await assert.rejects(
      reloadPageAndWaitForNewMainDocument(page, withDeadline, 1234),
      failure === "reload" ? /reload failure/ : /document deadline/,
    );
    assert.equal(listener, null);
    assert.equal(removed, 1);
    assert.equal(deadlineSettled, true);
  }
});

test("RemoteBrowser login waits for visibility and rejects origin drift before credentials", async () => {
  let entryReads = 0;
  let credentialMutations = 0;
  let submits = 0;
  const page = {
    async call(fn, ...args) {
      if (fn === readRemoteEntryState) {
        entryReads += 1;
        return entryReads === 1 ? null : { kind: "login" };
      }
      if (fn === fillRemoteLoginCredentials) {
        credentialMutations += 1;
        return { kind: "credentials-filled" };
      }
      if (fn === submitRemoteLogin) {
        submits += 1;
        return { kind: "submitted" };
      }
      if (fn === readRemoteReadyState) return { kind: "ready" };
      throw new Error(`unexpected page call with ${args.length} argument(s)`);
    },
  };
  const waitUntil = async (_label, predicate) => {
    for (let attempt = 0; attempt < 3; attempt += 1) {
      const value = await predicate();
      if (value) return value;
    }
    throw new Error("synthetic wait exhausted");
  };
  await loginAndroidRemote(page, "https://remote.test", "user", "password", waitUntil);
  assert.equal(entryReads, 2);
  assert.equal(credentialMutations, 1);
  assert.equal(submits, 1);

  let wrongOriginCredentialMutations = 0;
  const wrongOriginPage = {
    async call(fn) {
      if (fn === readRemoteEntryState) return { kind: "login" };
      if (fn === fillRemoteLoginCredentials) return { kind: "unexpected-origin" };
      wrongOriginCredentialMutations += 1;
      return { kind: "unexpected-call" };
    },
  };
  await assert.rejects(
    loginAndroidRemote(
      wrongOriginPage,
      "https://remote.test",
      "user",
      "password",
      waitUntil,
    ),
    /credentials rejected: unexpected-origin/,
  );
  assert.equal(wrongOriginCredentialMutations, 0);

  let wrongOriginSubmits = 0;
  const submitDriftPage = {
    async call(fn) {
      if (fn === readRemoteEntryState) return { kind: "login" };
      if (fn === fillRemoteLoginCredentials) return { kind: "credentials-filled" };
      if (fn === submitRemoteLogin) return { kind: "unexpected-origin" };
      wrongOriginSubmits += 1;
      return { kind: "unexpected-call" };
    },
  };
  await assert.rejects(
    loginAndroidRemote(
      submitDriftPage,
      "https://remote.test",
      "user",
      "password",
      waitUntil,
    ),
    /login submit rejected: unexpected-origin/,
  );
  assert.equal(wrongOriginSubmits, 0);

  const readyDriftPage = {
    async call(fn) {
      if (fn === readRemoteEntryState) return { kind: "ready" };
      if (fn === readRemoteReadyState) return { kind: "unexpected-origin" };
      return { kind: "unexpected-call" };
    },
  };
  await assert.rejects(
    loginAndroidRemote(
      readyDriftPage,
      "https://remote.test",
      "user",
      "password",
      waitUntil,
    ),
    /ready rejected: unexpected-origin/,
  );
});

test("timed-out CDP target discovery aborts its fetch", async () => {
  const originalFetch = globalThis.fetch;
  let observedSignal;
  globalThis.fetch = async (_url, { signal }) => {
    observedSignal = signal;
    return new Promise((_resolve, reject) => {
      signal.addEventListener("abort", () => reject(new Error("aborted")), { once: true });
    });
  };
  try {
    await assert.rejects(
      fetchCdpTargets("http://android.test", async (_label, promise) => {
        promise.catch(() => {});
        throw new Error("synthetic discovery timeout");
      }, 25),
      /synthetic discovery timeout/,
    );
    assert.equal(observedSignal.aborted, true);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const read = (path) => fs.readFileSync(new URL(path, import.meta.url), "utf8");
const initializeSource = read("../apps/mobile/src/embedded_backend/webview_bootstrap_init.js");
const prepareSource = read("../apps/mobile/src/embedded_backend/android_initial_session_prepare.js");
const initialize = (0, eval)(initializeSource);
const prepare = (0, eval)(prepareSource);
const installId = "0123456789abcdef0123456789abcdef";
const otherInstallId = "fedcba9876543210fedcba9876543210";
const fallback = {
  http_base: "http://127.0.0.1:40123",
  ws_base: "ws://127.0.0.1:40123",
  node_role: "main",
  session_bound: true,
  platform_lifecycle_authority: "native",
  capabilities: { backend_preference_control: true },
};

test("Rust wrapper composition keeps both injected sources executable", () => {
  const source = `(()=>{const init=${initializeSource};init(window,${JSON.stringify(fallback)},${JSON.stringify(installId)},false);})();`
    + `(()=>{const prepare=${prepareSource};prepare(window,${JSON.stringify(installId)});})();`;
  assert.doesNotThrow(() => new Function("window", source));
});

function storageWith(raw, behavior = {}) {
  const values = new Map();
  if (raw !== undefined) values.set("__DEVE_NATIVE_BOOTSTRAP_CURRENT__", raw);
  return {
    getItem(key) {
      if (behavior.getThrows) throw new Error("storage get failed");
      return values.get(key) ?? null;
    },
    setItem(key, value) {
      if (behavior.setThrows) throw new Error("storage set failed");
      values.set(key, value);
    },
    snapshot(key) {
      return values.get(key) ?? null;
    },
  };
}

function envelope(bootstrap = fallback, sessionInstallId = installId) {
  return JSON.stringify({ session_install_id: sessionInstallId, bootstrap });
}

function bootstrapRoot(storage) {
  const events = [];
  return {
    root: {
      sessionStorage: storage,
      Event,
      dispatchEvent: (event) => events.push(event.type),
    },
    events,
  };
}

for (const [name, raw] of [
  ["invalid JSON", "{bad"],
  ["boolean bootstrap", envelope(true)],
  ["wrong ready state", envelope({ ...fallback, session_bound: false })],
  ["wrong role", envelope({ ...fallback, node_role: "peer" })],
  ["missing lifecycle authority", envelope({ ...fallback, platform_lifecycle_authority: undefined })],
  ["wrong lifecycle authority", envelope({ ...fallback, platform_lifecycle_authority: "browser" })],
  ["wrong capability", envelope({ ...fallback, capabilities: { backend_preference_control: false } })],
  ["extra field", envelope({ ...fallback, session_material: "must-not-project" })],
  ["outer extra field", JSON.stringify({
    session_install_id: installId,
    bootstrap: fallback,
    session_material: "must-not-retain",
  })],
  ["old process identity", envelope(fallback, otherInstallId)],
]) {
  test(`native bootstrap replaces ${name} with the current fallback`, () => {
    const harness = bootstrapRoot(storageWith(raw));
    initialize(harness.root, fallback, installId, false);
    assert.equal(harness.root.__DEVE_NATIVE_SESSION_STORAGE_READY, true);
    assert.equal(harness.root.__DEVE_NATIVE_SESSION_INSTALL_ID, installId);
    assert.deepEqual(harness.root.__DEVE_NATIVE_BOOTSTRAP, fallback);
    assert.deepEqual(harness.events, []);
    assert.doesNotMatch(
      harness.root.sessionStorage.snapshot("__DEVE_NATIVE_BOOTSTRAP_CURRENT__"),
      /must-not/,
    );
  });
}

test("replacement source overwrites an older same-process endpoint", () => {
  const previous = { ...fallback, http_base: "http://127.0.0.1:40124", ws_base: "ws://127.0.0.1:40124" };
  const harness = bootstrapRoot(storageWith(envelope(previous)));
  initialize(harness.root, fallback, installId, true);
  assert.equal(harness.root.__DEVE_NATIVE_SESSION_STORAGE_READY, true);
  assert.deepEqual(harness.root.__DEVE_NATIVE_BOOTSTRAP, fallback);
  assert.deepEqual(harness.events, []);
});

test("reload preserves a validated later-generation endpoint from the same process", () => {
  const later = { ...fallback, http_base: "http://127.0.0.1:40124", ws_base: "ws://127.0.0.1:40124" };
  const harness = bootstrapRoot(storageWith(envelope(later)));
  initialize(harness.root, fallback, installId, false);
  assert.equal(harness.root.__DEVE_NATIVE_SESSION_STORAGE_READY, true);
  assert.deepEqual(harness.root.__DEVE_NATIVE_BOOTSTRAP, later);
  assert.deepEqual(harness.events, []);
});

for (const [operation, behavior] of [["read", { getThrows: true }], ["write", { setThrows: true }]]) {
  test(`unavailable native bootstrap storage ${operation} fails closed`, () => {
    const harness = bootstrapRoot(storageWith(undefined, behavior));
    initialize(harness.root, fallback, installId, false);
    assert.equal(harness.root.__DEVE_NATIVE_SESSION_STORAGE_READY, false);
    assert.deepEqual(harness.root.__DEVE_NATIVE_BOOTSTRAP, {
      service_state: "session_invalid",
      platform_lifecycle_authority: "native",
      capabilities: fallback.capabilities,
    });
    assert.deepEqual(harness.events, ["deve-native-service-error"]);
  });
}

test("replacement storage failure projects session invalid", () => {
  const harness = bootstrapRoot(storageWith(undefined, { setThrows: true }));
  initialize(harness.root, fallback, installId, true);
  assert.equal(harness.root.__DEVE_NATIVE_SESSION_STORAGE_READY, false);
  assert.equal(harness.root.__DEVE_NATIVE_BOOTSTRAP.service_state, "session_invalid");
  assert.deepEqual(harness.events, ["deve-native-service-error"]);
});

function prepareRoot(storage) {
  const queued = [];
  const events = [];
  let invokes = 0;
  let reloads = 0;
  return {
    root: {
      sessionStorage: storage,
      __DEVE_NATIVE_BOOTSTRAP: fallback,
      __DEVE_NATIVE_SESSION_STORAGE_READY: true,
      __TAURI_INTERNALS__: { invoke: async () => { invokes += 1; } },
      Event,
      dispatchEvent: (event) => events.push(event.type),
      queueMicrotask: (callback) => queued.push(callback),
      location: { reload: () => { reloads += 1; } },
    },
    queued,
    observation: () => ({ invokes, reloads, events }),
  };
}

test("storage admission failure does not invoke or reload", () => {
  const harness = prepareRoot(storageWith());
  harness.root.__DEVE_NATIVE_SESSION_STORAGE_READY = false;
  prepare(harness.root, installId);
  assert.deepEqual(harness.observation(), {
    invokes: 0,
    reloads: 0,
    events: ["deve-native-service-error"],
  });
  assert.deepEqual(harness.root.__DEVE_NATIVE_BOOTSTRAP, {
    service_state: "session_invalid",
    platform_lifecycle_authority: "native",
    capabilities: fallback.capabilities,
  });
});

test("installed-marker read failure does not invoke or reload", () => {
  const harness = prepareRoot(storageWith(undefined, { getThrows: true }));
  prepare(harness.root, installId);
  assert.deepEqual(harness.observation(), {
    invokes: 0,
    reloads: 0,
    events: ["deve-native-service-error"],
  });
});

test("confirmed marker skips native handoff and reload", () => {
  const storage = storageWith();
  storage.setItem("__DEVE_NATIVE_SESSION_INSTALLED__", installId);
  const harness = prepareRoot(storage);
  prepare(harness.root, installId);
  assert.deepEqual(harness.observation(), { invokes: 0, reloads: 0, events: [] });
});

test("successful handoff confirms the marker before one reload", async () => {
  const harness = prepareRoot(storageWith());
  prepare(harness.root, installId);
  assert.equal(harness.queued.length, 1);
  await harness.queued[0]();
  assert.deepEqual(harness.observation(), { invokes: 1, reloads: 1, events: [] });
});

test("rejected native handoff does not reload", async () => {
  const harness = prepareRoot(storageWith());
  let rejectedInvokes = 0;
  harness.root.__TAURI_INTERNALS__.invoke = async () => {
    rejectedInvokes += 1;
    throw new Error("native handoff rejected");
  };
  prepare(harness.root, installId);
  await harness.queued[0]();
  assert.equal(rejectedInvokes, 1);
  assert.equal(harness.observation().reloads, 0);
  assert.deepEqual(harness.observation().events, ["deve-native-service-error"]);
});

test("marker storage failure after handoff does not reload", async () => {
  const harness = prepareRoot(storageWith(undefined, { setThrows: true }));
  prepare(harness.root, installId);
  await harness.queued[0]();
  assert.deepEqual(harness.observation(), {
    invokes: 1,
    reloads: 0,
    events: ["deve-native-service-error"],
  });
});

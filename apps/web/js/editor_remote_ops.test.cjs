const assert = require("node:assert/strict");
const path = require("node:path");
const esbuild = require("esbuild");
const { EditorState } = require("@codemirror/state");

async function loadModule() {
  const result = await esbuild.build({
    entryPoints: [path.join(__dirname, "editor_remote_ops.js")],
    bundle: true,
    format: "cjs",
    platform: "node",
    write: false,
    plugins: [{
      name: "editor-remote-ops-test-mocks",
      setup(build) {
        build.onResolve({ filter: /^@codemirror\/(view|state)$/ }, (args) => ({
          path: args.path,
          namespace: "mock",
        }));
        build.onResolve({ filter: /editor_state\.js$/ }, () => ({
          path: "editor_state.js",
          namespace: "mock",
        }));
        build.onLoad({ filter: /.*/, namespace: "mock" }, (args) => {
          if (args.path === "@codemirror/view") {
            return { contents: "export const EditorView = { scrollIntoView() {} };" };
          }
          if (args.path === "@codemirror/state") {
            return {
              contents: [
                "export const EditorState = { readOnly: { of() {} } };",
                "export const Transaction = { addToHistory: { of(value) { return { addToHistory: value }; } } };",
              ].join("\n"),
            };
          }
          return { contents: "export const ctx = globalThis.__DEVE_EDITOR_REMOTE_OPS_TEST_CTX__;" };
        });
      },
    }],
  });
  const module = { exports: {} };
  new Function("module", "exports", "require", result.outputFiles[0].text)(
    module,
    module.exports,
    require,
  );
  return module.exports;
}

function testView(content) {
  const calls = [];
  const view = {
    state: { doc: { length: content.length, toString: () => content } },
    contentDOM: { setAttribute() {} },
    dispatch(...specs) {
      calls.push(specs);
      for (const spec of specs) {
        const change = spec.changes;
        if (!change) continue;
        content = content.slice(0, change.from) + change.insert + content.slice(change.to ?? change.from);
      }
      this.state.doc = { length: content.length, toString: () => content };
    },
  };
  return { view, calls, content: () => content };
}

(async () => {
  const valid = testView("abcd");
  globalThis.__DEVE_EDITOR_REMOTE_OPS_TEST_CTX__ = {
    activeView: valid.view,
    isRemote: false,
    readOnlyCompartment: { reconfigure() {} },
  };
  const remoteOps = await loadModule();

  assert.equal(remoteOps.applyRemoteOpsBatch(JSON.stringify([
    { Insert: { pos: 4, content: "XY" } },
    { Delete: { pos: 1, len: 3 } },
  ])), true);
  assert.equal(valid.calls.length, 1, "valid batch must use one dispatch call");
  assert.equal(valid.calls[0].length, 2);
  assert.ok(valid.calls[0].every((spec) => spec.sequential === true));
  assert.ok(valid.calls[0].every((spec) => spec.annotations.addToHistory === false));
  assert.equal(valid.content(), "aXY");

  const realState = EditorState.create({ doc: "abcd" });
  const realSpecs = remoteOps.buildRemoteBatchSpecs([
    { Insert: { pos: 4, content: "XY" } },
    { Delete: { pos: 1, len: 3 } },
  ], realState.doc.length, []);
  assert.equal(
    realState.update(...realSpecs).newDoc.toString(),
    "aXY",
    "real CodeMirror state must honor sequential batch coordinates",
  );

  const invalid = testView("abcd");
  globalThis.__DEVE_EDITOR_REMOTE_OPS_TEST_CTX__.activeView = invalid.view;
  const originalConsoleError = console.error;
  console.error = () => {};
  try {
    assert.equal(remoteOps.applyRemoteOpsBatch(JSON.stringify([
      { Insert: { pos: 4, content: "ok" } },
      { Delete: { pos: 99, len: 1 } },
    ])), false);
  } finally {
    console.error = originalConsoleError;
  }
  assert.equal(invalid.calls.length, 0, "batch validation failure must not dispatch a prefix");
  assert.equal(invalid.content(), "abcd");

  assert.throws(
    () => remoteOps.buildRemoteBatchSpecs([{ Insert: { pos: 5, content: "x" } }], 4, {}),
    /Invalid remote op range/,
  );
  assert.throws(
    () => remoteOps.buildRemoteBatchSpecs([{ Insert: { pos: 0 } }], 4, {}),
    /insert content must be a string/i,
  );
  assert.throws(
    () => remoteOps.buildRemoteBatchSpecs([{ Delete: { pos: 0, len: Number.MAX_SAFE_INTEGER + 1 } }], 4, {}),
    /Invalid remote delete length/,
  );
  const activeHost = {};
  const staleHost = {};
  const ownerScoped = testView("owned");
  globalThis.__DEVE_EDITOR_REMOTE_OPS_TEST_CTX__.activeView = ownerScoped.view;
  globalThis.__DEVE_EDITOR_REMOTE_OPS_TEST_CTX__.activeHost = activeHost;
  assert.equal(
    remoteOps.setReadOnlyForHost(staleHost, false),
    false,
    "stale hosts must not mutate the active editor readonly compartment",
  );
  assert.equal(ownerScoped.calls.length, 0);
  assert.equal(remoteOps.setReadOnlyForHost(activeHost, true), true);
  assert.equal(ownerScoped.calls.length, 1);
  console.log("editor-remote-ops-atomic-batch: ok");
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const root = __dirname;
const registrySource = fs.readFileSync(path.join(root, "web_bridge_registry.js"), "utf8");
const editorAdapterSource = fs.readFileSync(path.join(root, "editor_adapter.js"), "utf8");
const gutterDiffSource = fs.readFileSync(path.join(root, "extensions", "gutter_diff.js"), "utf8");
const indexHtmlSource = fs.readFileSync(path.join(root, "..", "index.html"), "utf8");

const adapterBridgeNames = [
  "setupCodeMirror",
  "destroyEditor",
  "getEditorContent",
  "applyRemoteContent",
  "applyRemoteOp",
  "applyRemoteOpsBatch",
  "syncEditorStateToRust",
  "scrollGlobal",
  "setReadOnly",
  "updateGutterDiff",
  "getEditorSelection",
  "mobileUndo",
  "mobileRedo",
];

const indexBridgeNames = [
  "setupCodeMirror",
  "destroyEditor",
  "getEditorContent",
  "applyRemoteContent",
  "applyRemoteOp",
  "applyRemoteOpsBatch",
  "scrollGlobal",
  "setReadOnly",
  "updateGutterDiff",
  "getEditorSelection",
  "mobileUndo",
  "mobileRedo",
  "mobileInsertText",
  "mobileWrapSelection",
  "ensureEditorAdapter",
  "queueEditorAction",
  "queueEditorMount",
];

const registryContext = { window: {} };
vm.runInNewContext(registrySource, registryContext, { filename: "web_bridge_registry.js" });
assert.equal(
  typeof registryContext.window.__deveWebBridge?.register,
  "function",
  "web bridge registry must expose register()"
);
const registeredValue = registryContext.window.__deveWebBridge.register(
  "setupCodeMirror",
  () => true,
  { runtime: "render_projection_runtime", source: "test" }
);
assert.equal(registryContext.window.setupCodeMirror, registeredValue);
assert.equal(
  JSON.stringify(registryContext.window.__deveWebBridge.describe()),
  JSON.stringify([
    {
      name: "setupCodeMirror",
      meta: { runtime: "render_projection_runtime", source: "test" },
    },
  ])
);

assert.match(
  editorAdapterSource,
  /function registerBrowserBridgeGlobal\(/,
  "editor adapter must expose globals through the bridge helper"
);
assert.match(
  editorAdapterSource,
  /runtime:\s*"render_projection_runtime"/,
  "editor bridge globals must default to the render projection boundary"
);
assert.match(
  editorAdapterSource,
  /authority:\s*"none"/,
  "editor bridge globals must not claim authority ownership"
);
assert.doesNotMatch(
  editorAdapterSource,
  /target\[name\]\s*=\s*value/,
  "missing web bridge registry must fail closed instead of silently assigning globals"
);

for (const name of adapterBridgeNames) {
  assert.match(
    editorAdapterSource,
    new RegExp(`registerBrowserBridgeGlobal\\("${name}"`),
    `${name} must be registered through the browser bridge registry`
  );
  assert.doesNotMatch(
    editorAdapterSource,
    new RegExp(`(?:window|globalThis)\\.${name}\\s*=`),
    `${name} must not be assigned directly on window/globalThis`
  );
}

assert.match(
  indexHtmlSource,
  /const registerEditorBridgeGlobal = \(name, value, meta = \{\}\) =>/,
  "index bootstrap must use the bridge registry helper"
);
assert.match(
  indexHtmlSource,
  /web bridge registry unavailable before registering/,
  "index bootstrap must fail closed when the bridge registry is missing"
);

for (const name of indexBridgeNames) {
  assert.match(
    indexHtmlSource,
    new RegExp(`registerEditorBridgeGlobal\\("${name}"`),
    `${name} wrapper must be registered through the browser bridge registry`
  );
  assert.doesNotMatch(
    indexHtmlSource,
    new RegExp(`window\\.${name}\\s*=`),
    `${name} wrapper must not be assigned directly in index.html`
  );
}

assert.doesNotMatch(
  gutterDiffSource,
  /window\.updateGutterDiff\s*=/,
  "gutter diff extension must not bypass the adapter bridge registration"
);

console.log("web-bridge-registry-editor-globals: ok");

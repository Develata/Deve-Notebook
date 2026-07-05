const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const root = __dirname;
const registrySource = fs.readFileSync(path.join(root, "web_bridge_registry.js"), "utf8");
const editorAdapterSource = fs.readFileSync(path.join(root, "editor_adapter.js"), "utf8");
const chatMathBootstrapSource = fs.readFileSync(path.join(root, "chat_math_bootstrap.js"), "utf8");
const chatMathSource = fs.readFileSync(path.join(root, "chat_math.js"), "utf8");
const gutterDiffSource = fs.readFileSync(path.join(root, "extensions", "gutter_diff.js"), "utf8");
const initSource = fs.readFileSync(path.join(root, "init.js"), "utf8");
const codeMenuSource = fs.readFileSync(path.join(root, "extensions", "code_menu.js"), "utf8");
const nativeBackendBridgeSource = fs.readFileSync(
  path.join(root, "native_backend_bridge.js"),
  "utf8"
);
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
  "mobileInsertText",
  "mobileWrapSelection",
  "mobileUndo",
  "mobileRedo",
];

const indexBridgeNames = [
  "__deveEditorBootstrap",
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

const indexBootBridgeNames = [
  "__DEVE_DEBUG_OVERLAY__",
  "showOverlay",
  "setBootPanel",
  "hideBootPanel",
  "hideOverlay",
  "logToOverlay",
];

const initBridgeNames = ["deve_code_actions", "deve_i18n"];

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

const initRuntimeContext = { window: {} };
vm.runInNewContext(registrySource, initRuntimeContext, {
  filename: "web_bridge_registry.js",
});
vm.runInNewContext(initSource, initRuntimeContext, { filename: "init.js" });
const initEntries = initRuntimeContext.window.__deveWebBridge
  .describe()
  .filter((entry) => initBridgeNames.includes(entry.name));
assert.equal(
  JSON.stringify(initEntries),
  JSON.stringify([
    {
      name: "deve_code_actions",
      meta: {
        runtime: "widget_bridge_runtime",
        source: "init-bootstrap",
        authority: "none",
        role: "code-toolbar-action-registry",
      },
    },
    {
      name: "deve_i18n",
      meta: {
        runtime: "widget_bridge_runtime",
        source: "init-bootstrap",
        authority: "none",
        role: "browser-i18n-copy-registry",
      },
    },
  ]),
  "init bootstrap globals must be registered through the browser bridge registry"
);
assert.equal(
  Array.isArray(initRuntimeContext.window.deve_code_actions),
  true,
  "init bootstrap must expose a code action array through the registry"
);
assert.equal(
  initRuntimeContext.window.deve_i18n.editor.noActionsAvailable,
  "No actions available",
  "init bootstrap must expose editor i18n copy through the registry"
);
assert.throws(
  () => vm.runInNewContext(initSource, { window: {} }, { filename: "init.js" }),
  /web bridge registry unavailable before registering deve_code_actions/,
  "init bootstrap must fail closed without the registry"
);

const nativeBridgeRuntimeSource = nativeBackendBridgeSource.replace(
  'import { invoke } from "@tauri-apps/api/core";',
  "const invoke = async (command, args) => ({ command, args });"
);
const nativeBridgeRuntimeContext = {
  window: {
    __TAURI_INTERNALS__: {
      invoke: () => undefined,
    },
  },
};
vm.runInNewContext(registrySource, nativeBridgeRuntimeContext, {
  filename: "web_bridge_registry.js",
});
vm.runInNewContext(nativeBridgeRuntimeSource, nativeBridgeRuntimeContext, {
  filename: "native_backend_bridge.js",
});
const nativeBackendEntries = nativeBridgeRuntimeContext.window.__deveWebBridge
  .describe()
  .filter((entry) => entry.name === "__DEVE_NATIVE_BACKEND_CONFIG__");
assert.equal(
  JSON.stringify(nativeBackendEntries),
  JSON.stringify([
    {
      name: "__DEVE_NATIVE_BACKEND_CONFIG__",
      meta: {
        runtime: "native_shell_mode_runtime",
        source: "native-backend-bridge",
        authority: "none",
        role: "host-local-backend-preference-facade",
      },
    },
  ]),
  "native backend config facade must be registered through the browser bridge registry"
);
assert.equal(
  nativeBridgeRuntimeContext.window.__DEVE_NATIVE_BACKEND_CONFIG__.available(),
  true,
  "native backend facade must preserve Tauri invoke availability probing"
);
assert.throws(
  () => vm.runInNewContext(nativeBridgeRuntimeSource, { window: {} }, {
    filename: "native_backend_bridge.js",
  }),
  /web bridge registry unavailable before registering native backend config/,
  "native backend bridge must fail closed without the registry"
);

const chatMathRuntimeContext = { window: {} };
vm.runInNewContext(registrySource, chatMathRuntimeContext, {
  filename: "web_bridge_registry.js",
});
vm.runInNewContext(chatMathBootstrapSource, chatMathRuntimeContext, {
  filename: "chat_math_bootstrap.js",
});
vm.runInNewContext(chatMathSource, chatMathRuntimeContext, {
  filename: "chat_math.js",
});
const chatMathEntries = chatMathRuntimeContext.window.__deveWebBridge
  .describe()
  .filter((entry) => ["renderChatMath", "__deveChatMath"].includes(entry.name));
assert.equal(
  JSON.stringify(chatMathEntries),
  JSON.stringify([
    {
      name: "renderChatMath",
      meta: {
        runtime: "render_projection_runtime",
        source: "chat_math",
        authority: "none",
        role: "chat-math-render-pass",
      },
    },
    {
      name: "__deveChatMath",
      meta: {
        runtime: "render_projection_runtime",
        source: "chat_math",
        authority: "none",
        role: "test-support",
      },
    },
  ]),
  "chat math scripts must preserve registry ownership after ordered loading"
);
assert.throws(
  () => vm.runInNewContext(chatMathBootstrapSource, { window: {} }, {
    filename: "chat_math_bootstrap.js",
  }),
  /web bridge registry unavailable before registering renderChatMath/,
  "chat math bootstrap must fail closed without the registry"
);
assert.throws(
  () => vm.runInNewContext(chatMathSource, { window: {} }, {
    filename: "chat_math.js",
  }),
  /web bridge registry unavailable before registering chat math/,
  "chat math must fail closed without the registry"
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
assert.match(
  indexHtmlSource,
  /registerEditorBridgeGlobal\("__deveEditorBootstrap", editorBootstrapState,\s*\{\s*runtime:\s*"widget_bridge_runtime"/,
  "index editor bootstrap state must be registered through the bridge registry"
);
assert.match(
  indexHtmlSource,
  /role:\s*"editor-bootstrap-state"/,
  "index editor bootstrap state must declare its bridge role"
);
assert.doesNotMatch(
  indexHtmlSource,
  /window\._debug_view\b/,
  "index mobile editor stubs must not read the adapter-owned CodeMirror view directly"
);
for (const name of [
  "_editor_queue",
  "_pending_editor_actions",
  "_cm_loaded",
  "_editor_bridge_ready",
  "_realInit",
  "_adapter_loading",
]) {
  assert.doesNotMatch(
    indexHtmlSource,
    new RegExp(`window\\.${name}\\b`),
    `${name} bootstrap state must not be stored directly on window`
  );
}
assert.match(
  indexHtmlSource,
  /const registerIndexBridgeGlobal = \(name, value, meta = \{\}\) =>/,
  "index boot helpers must use the bridge registry helper"
);
const indexBootHelperMatch = indexHtmlSource.match(
  /const registerIndexBridgeGlobal = \(name, value, meta = \{\}\) => \{[\s\S]*?\n      \};/
);
assert.ok(indexBootHelperMatch, "index boot helper registration block must be present");
const indexBootHelperSource = indexBootHelperMatch[0];
assert.match(
  indexBootHelperSource,
  /source:\s*"index-boot-bootstrap"/,
  "index boot helper bridge entries must declare their bootstrap source"
);
assert.match(
  indexBootHelperSource,
  /authority:\s*"none"/,
  "index boot helper bridge entries must not claim authority ownership"
);

for (const name of indexBootBridgeNames) {
  assert.match(
    indexHtmlSource,
    new RegExp(`registerIndexBridgeGlobal\\("${name}"`),
    `${name} boot helper must be registered through the browser bridge registry`
  );
  assert.doesNotMatch(
    indexHtmlSource,
    new RegExp(`window\\.${name}\\s*=`),
    `${name} boot helper must not be assigned directly in index.html`
  );
}

assert.doesNotMatch(
  indexHtmlSource,
  /window\.onerror\s*=/,
  "index boot error handling must not assign window.onerror directly"
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

assert.match(
  chatMathBootstrapSource,
  /registerFallback\("renderChatMath"/,
  "chat math bootstrap fallback must be registered through the bridge registry"
);
assert.match(
  chatMathBootstrapSource,
  /web bridge registry unavailable before registering renderChatMath/,
  "chat math bootstrap must fail closed when the bridge registry is missing"
);
assert.doesNotMatch(
  chatMathBootstrapSource,
  /window\.renderChatMath\s*=/,
  "chat math bootstrap must not assign renderChatMath directly"
);

for (const name of ["renderChatMath", "__deveChatMath"]) {
  assert.match(
    chatMathSource,
    new RegExp(`bridge\\.register\\("${name}"`),
    `${name} must be registered through the browser bridge registry`
  );
  assert.doesNotMatch(
    chatMathSource,
    new RegExp(`window\\.${name}\\s*=`),
    `${name} must not be assigned directly by chat_math.js`
  );
}
assert.match(
  chatMathSource,
  /web bridge registry unavailable before registering chat math/,
  "chat math must fail closed when the bridge registry is missing"
);
assert.match(
  chatMathSource,
  /authority:\s*"none"/,
  "chat math bridge entries must not claim authority ownership"
);

assert.match(
  nativeBackendBridgeSource,
  /bridge\.register\("__DEVE_NATIVE_BACKEND_CONFIG__"/,
  "native backend config facade must be registered through the browser bridge registry"
);
assert.match(
  nativeBackendBridgeSource,
  /runtime:\s*"native_shell_mode_runtime"/,
  "native backend config facade must declare the native shell mode runtime"
);
assert.match(
  nativeBackendBridgeSource,
  /authority:\s*"none"/,
  "native backend config facade must not claim authority ownership"
);
assert.doesNotMatch(
  nativeBackendBridgeSource,
  /window\.__DEVE_NATIVE_BACKEND_CONFIG__\s*=/,
  "native backend config facade must not be assigned directly on window"
);

assert.match(
  initSource,
  /registerInitBridgeGlobal\("deve_code_actions"/,
  "code toolbar action registry must be registered through the browser bridge registry"
);
assert.match(
  initSource,
  /registerInitBridgeGlobal\("deve_i18n"/,
  "browser i18n copy registry must be registered through the browser bridge registry"
);
assert.doesNotMatch(
  initSource,
  /window\.deve_code_actions\s*=/,
  "init bootstrap must not assign deve_code_actions directly"
);
assert.doesNotMatch(
  initSource,
  /window\.deve_i18n\s*=/,
  "init bootstrap must not assign deve_i18n directly"
);
assert.doesNotMatch(
  codeMenuSource,
  /window\.deve_code_actions\s*=/,
  "code menu must not initialize the code action registry directly"
);
assert.match(
  codeMenuSource,
  /Array\.isArray\(window\.deve_code_actions\)/,
  "code menu may only read the bridge-registered action registry"
);

const registryScriptIndex = indexHtmlSource.indexOf('src="js/web_bridge_registry.js"');
const initScriptIndex = indexHtmlSource.indexOf('src="js/init.js"');
const lazyEditorBundleIndex = indexHtmlSource.indexOf('import("./js/editor.bundle.js');
assert.ok(registryScriptIndex >= 0, "index.html must load web_bridge_registry.js");
assert.ok(initScriptIndex >= 0, "index.html must load init.js");
assert.ok(lazyEditorBundleIndex >= 0, "index.html must lazy-load the editor bundle");
assert.ok(
  registryScriptIndex < initScriptIndex && initScriptIndex < lazyEditorBundleIndex,
  "index script order must load registry before init and init before the lazy editor adapter"
);

console.log("web-bridge-registry-editor-globals: ok");

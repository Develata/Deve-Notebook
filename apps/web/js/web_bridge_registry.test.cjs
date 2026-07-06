const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const root = __dirname;
const registrySource = fs.readFileSync(path.join(root, "web_bridge_registry.js"), "utf8");
const katexBridgeSource = fs.readFileSync(path.join(root, "katex_bridge.js"), "utf8");
const renderingBridgeSource = fs.readFileSync(path.join(root, "rendering_bridge.js"), "utf8");
const widgetBridgeSource = fs.readFileSync(path.join(root, "widget_bridge.js"), "utf8");
const editorAdapterSource = fs.readFileSync(path.join(root, "editor_adapter.js"), "utf8");
const chatMathBootstrapSource = fs.readFileSync(path.join(root, "chat_math_bootstrap.js"), "utf8");
const chatMathSource = fs.readFileSync(path.join(root, "chat_math.js"), "utf8");
const gutterDiffSource = fs.readFileSync(path.join(root, "extensions", "gutter_diff.js"), "utf8");
const mathExtensionSource = fs.readFileSync(path.join(root, "extensions", "math.js"), "utf8");
const inlineRendererSource = fs.readFileSync(
  path.join(root, "extensions", "inline_renderer.js"),
  "utf8"
);
const imageExtensionSource = fs.readFileSync(path.join(root, "extensions", "image.js"), "utf8");
const initSource = fs.readFileSync(path.join(root, "init.js"), "utf8");
const i18nSource = fs.readFileSync(path.join(root, "i18n.js"), "utf8");
const codeMenuSource = fs.readFileSync(path.join(root, "extensions", "code_menu.js"), "utf8");
const nativeBackendBridgeSource = fs.readFileSync(
  path.join(root, "native_backend_bridge.js"),
  "utf8"
);
const outlineKatexSource = fs.readFileSync(
  path.join(root, "..", "src", "components", "outline_render", "katex.rs"),
  "utf8"
);
const webMainSource = fs.readFileSync(path.join(root, "..", "src", "main.rs"), "utf8");
const indexHtmlSource = fs.readFileSync(path.join(root, "..", "index.html"), "utf8");
const indexThemeBootstrapSource = fs.readFileSync(
  path.join(root, "index_theme_bootstrap.js"),
  "utf8"
);
const indexBootstrapSource = fs.readFileSync(path.join(root, "index_bootstrap.js"), "utf8");
const indexEditorAdapterSource = fs.readFileSync(
  path.join(root, "index_editor_adapter.js"),
  "utf8"
);
const indexBridgeSource = `${indexBootstrapSource}\n${indexEditorAdapterSource}`;

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
assert.equal(
  registryContext.window.__deveWebBridge?.policyVersion,
  "projection-only-v1",
  "web bridge registry must expose its projection-only admission policy version"
);
assert.equal(
  typeof registryContext.window.__deveWebBridge?.get,
  "function",
  "web bridge registry must expose get()"
);
assert.equal(
  typeof registryContext.window.__deveWebBridge?.call,
  "function",
  "web bridge registry must expose call()"
);
const registeredValue = registryContext.window.__deveWebBridge.register(
  "setupCodeMirror",
  () => true,
  { runtime: "render_projection_runtime", source: "test", authority: "none" }
);
assert.equal(registryContext.window.setupCodeMirror, registeredValue);
assert.equal(
  registryContext.window.__deveWebBridge.get("setupCodeMirror"),
  registeredValue,
  "registry get() must read registered globals through the bridge boundary"
);
assert.equal(
  registryContext.window.__deveWebBridge.call("setupCodeMirror"),
  true,
  "registry call() must invoke registered functions through the bridge boundary"
);
registryContext.window.setupCodeMirror = () => "overwritten";
assert.equal(
  registryContext.window.__deveWebBridge.get("setupCodeMirror"),
  registeredValue,
  "registry get() must not let later window overwrites replace the registered bridge value"
);
assert.equal(
  registryContext.window.__deveWebBridge.call("setupCodeMirror"),
  true,
  "registry call() must not dispatch to later direct window overwrites"
);
assert.throws(
  () => registryContext.window.__deveWebBridge.call("missingBridgeGlobal"),
  /web bridge global missingBridgeGlobal is not callable/,
  "registry call() must fail closed for missing bridge globals"
);
assert.equal(
  JSON.stringify(registryContext.window.__deveWebBridge.describe()),
  JSON.stringify([
    {
      name: "setupCodeMirror",
      meta: { runtime: "render_projection_runtime", source: "test", authority: "none" },
    },
  ])
);
assert.throws(
  () =>
    registryContext.window.__deveWebBridge.register(
      "commitAnchorState",
      () => true,
      { runtime: "render_projection_runtime", source: "test", authority: "ledger" },
    ),
  /web bridge global commitAnchorState must declare authority none/,
  "registry must reject globals that claim business authority"
);
assert.equal(
  typeof registryContext.window.commitAnchorState,
  "undefined",
  "rejected authority globals must not be written onto window"
);
assert.throws(
  () =>
    registryContext.window.__deveWebBridge.register(
      "safeFacadeName",
      () => true,
      {
        runtime: "widget_bridge_runtime",
        source: "test",
        authority: "none",
        role: "source-control-state",
      },
    ),
  /web bridge global safeFacadeName metadata must stay projection-only/,
  "registry must reject roles that imply Source Control or writer-state authority"
);
assert.equal(
  typeof registryContext.window.safeFacadeName,
  "undefined",
  "rejected authority roles must not be written onto window"
);
assert.throws(
  () =>
    registryContext.window.__deveWebBridge.register(
      "sourceControlState",
      () => true,
      {
        runtime: "widget_bridge_runtime",
        source: "test",
        authority: "none",
        role: "ui-facade",
      },
    ),
  /web bridge global sourceControlState metadata must stay projection-only/,
  "registry must reject names that imply Source Control or writer-state authority"
);
assert.equal(
  typeof registryContext.window.sourceControlState,
  "undefined",
  "rejected authority names must not be written onto window"
);
assert.throws(
  () =>
    registryContext.window.__deveWebBridge.register(
      "safeFacadeName",
      () => true,
      {
        runtime: "widget_bridge_runtime",
        source: "commitAnchorBootstrap",
        authority: "none",
        role: "ui-facade",
      },
    ),
  /web bridge global safeFacadeName metadata must stay projection-only/,
  "registry must reject sources that imply Source Control or writer-state authority"
);
assert.equal(
  typeof registryContext.window.safeFacadeName,
  "undefined",
  "rejected authority sources must not be written onto window"
);
for (const unsafeName of [
  "ledgerstate",
  "pendingstate",
  "ackstate",
  "rejectstate",
  "stagingarea",
  "backupstatus",
  "sourcecontrolState",
  "commitanchorState",
  "gitmirrorStatus",
  "pendingfsopsQueue",
  "remoteprojectionFacade",
]) {
  assert.throws(
    () =>
      registryContext.window.__deveWebBridge.register(
        unsafeName,
        () => true,
        {
          runtime: "widget_bridge_runtime",
          source: "test",
          authority: "none",
          role: "ui-facade",
        },
      ),
    new RegExp(`web bridge global ${unsafeName} metadata must stay projection-only`),
    "registry must reject collapsed authority names without relying on camel-case separators"
  );
  assert.equal(
    typeof registryContext.window[unsafeName],
    "undefined",
    "rejected collapsed authority names must not be written onto window"
  );
}
assert.throws(
  () =>
    registryContext.window.__deveWebBridge.register(
      "safeCollapsedSourceFacade",
      () => true,
      {
        runtime: "widget_bridge_runtime",
        source: "gitmirrorBootstrap",
        authority: "none",
        role: "ui-facade",
      },
    ),
  /web bridge global safeCollapsedSourceFacade metadata must stay projection-only/,
  "registry must reject collapsed authority sources"
);
assert.equal(
  typeof registryContext.window.safeCollapsedSourceFacade,
  "undefined",
  "rejected collapsed authority sources must not be written onto window"
);
assert.throws(
  () =>
    registryContext.window.__deveWebBridge.register(
      "safeCollapsedRoleFacade",
      () => true,
      {
        runtime: "widget_bridge_runtime",
        source: "test",
        authority: "none",
        role: "remoteprojection-status",
      },
    ),
  /web bridge global safeCollapsedRoleFacade metadata must stay projection-only/,
  "registry must reject collapsed authority roles"
);
assert.equal(
  typeof registryContext.window.safeCollapsedRoleFacade,
  "undefined",
  "rejected collapsed authority roles must not be written onto window"
);
const samePolicyFakeRegistryContext = {
  window: {
    __deveWebBridge: {
      policyVersion: "projection-only-v1",
      register(name, value) {
        this[name] = value;
        return value;
      },
      registerFallback() {},
      get() {},
      call() {},
      describe() {
        return [];
      },
    },
  },
};
vm.runInNewContext(registrySource, samePolicyFakeRegistryContext, {
  filename: "web_bridge_registry.js",
});
assert.throws(
  () =>
    samePolicyFakeRegistryContext.window.__deveWebBridge.register(
      "commitAnchorState",
      () => true,
      { runtime: "render_projection_runtime", source: "test", authority: "ledger" },
    ),
  /web bridge global commitAnchorState must declare authority none/,
  "same policy marker registries must still be revalidated by the local implementation"
);
const unsafeFallback = () => "unsafe";
const safeFallback = () => false;
const fallbackContext = { window: { renderChatMath: unsafeFallback } };
vm.runInNewContext(registrySource, fallbackContext, {
  filename: "web_bridge_registry.js",
});
const fallbackResult = fallbackContext.window.__deveWebBridge.registerFallback(
  "renderChatMath",
  safeFallback,
  {
    runtime: "render_projection_runtime",
    source: "chat_math_bootstrap",
    authority: "none",
    role: "chat-math-fallback",
  },
);
assert.equal(
  fallbackResult,
  safeFallback,
  "fallback registration must not adopt an unregistered preexisting window value"
);
assert.equal(
  fallbackContext.window.renderChatMath,
  safeFallback,
  "fallback registration must overwrite unregistered preexisting window values"
);
const trustedLegacyValue = () => "trusted";
const overwrittenLegacyValue = () => "overwritten";
const legacyRegistryContext = {
  window: {
    setupCodeMirror: overwrittenLegacyValue,
    __deveWebBridge: {
      register() {},
      registerFallback() {},
      get(name) {
        return name === "setupCodeMirror" ? trustedLegacyValue : undefined;
      },
      call() {},
      describe() {
        return [
          {
            name: "setupCodeMirror",
            meta: {
              runtime: "render_projection_runtime",
              source: "legacy",
              authority: "none",
            },
          },
        ];
      },
    },
  },
};
vm.runInNewContext(registrySource, legacyRegistryContext, {
  filename: "web_bridge_registry.js",
});
assert.equal(
  legacyRegistryContext.window.__deveWebBridge.policyVersion,
  "projection-only-v1",
  "legacy registries without the policy marker must be replaced by the projection-only registry"
);
assert.equal(
  JSON.stringify(legacyRegistryContext.window.__deveWebBridge.describe()),
  JSON.stringify([
    {
      name: "setupCodeMirror",
      meta: {
        runtime: "render_projection_runtime",
        source: "legacy",
        authority: "none",
        adopted: true,
      },
    },
  ]),
  "legacy registry entries must be adopted only after projection-only policy validation"
);
assert.equal(
  legacyRegistryContext.window.__deveWebBridge.call("setupCodeMirror"),
  "trusted",
  "legacy adoption must call the old registry value instead of an overwritten window value"
);
assert.equal(
  legacyRegistryContext.window.setupCodeMirror,
  trustedLegacyValue,
  "legacy adoption must restore the old registry value onto window"
);

const katexBridgeRuntimeContext = {
  window: {
    katex: {
      render(content, element, options) {
        element.rendered = { content, options };
      },
      renderToString(content, options) {
        return JSON.stringify({ content, options });
      },
    },
  },
};
vm.runInNewContext(registrySource, katexBridgeRuntimeContext, {
  filename: "web_bridge_registry.js",
});
vm.runInNewContext(katexBridgeSource, katexBridgeRuntimeContext, {
  filename: "katex_bridge.js",
});
const katexEntries = katexBridgeRuntimeContext.window.__deveWebBridge
  .describe()
  .filter((entry) => entry.name === "__deveKatex");
assert.equal(
  JSON.stringify(katexEntries),
  JSON.stringify([
    {
      name: "__deveKatex",
      meta: {
        runtime: "render_projection_runtime",
        source: "katex_bridge",
        authority: "none",
        role: "katex-render-facade",
      },
    },
  ]),
  "KaTeX facade must be registered through the browser bridge registry"
);
const katexFacade = katexBridgeRuntimeContext.window.__deveWebBridge.get("__deveKatex");
const katexTarget = {};
assert.equal(katexFacade.available(), true, "KaTeX facade must preserve availability probing");
assert.equal(
  katexFacade.render("a^2", katexTarget, { displayMode: true }),
  true,
  "KaTeX facade must report successful rendering"
);
assert.deepEqual(katexTarget.rendered, {
  content: "a^2",
  options: { displayMode: true },
});
assert.equal(
  katexFacade.renderToString("a^2", { displayMode: false }),
  JSON.stringify({ content: "a^2", options: { displayMode: false } }),
  "KaTeX facade must expose renderToString for Rust outline projection"
);
const missingKatexContext = { window: {} };
vm.runInNewContext(registrySource, missingKatexContext, {
  filename: "web_bridge_registry.js",
});
vm.runInNewContext(katexBridgeSource, missingKatexContext, {
  filename: "katex_bridge.js",
});
const missingKatexFacade = missingKatexContext.window.__deveWebBridge.get("__deveKatex");
assert.equal(missingKatexFacade.available(), false);
assert.equal(missingKatexFacade.render("x", {}, {}), false);
assert.equal(missingKatexFacade.renderToString("x", {}), null);
const throwingKatexContext = {
  window: {
    katex: {
      render() {
        throw new Error("render failed");
      },
      renderToString() {
        throw new Error("renderToString failed");
      },
    },
  },
};
vm.runInNewContext(registrySource, throwingKatexContext, {
  filename: "web_bridge_registry.js",
});
vm.runInNewContext(katexBridgeSource, throwingKatexContext, {
  filename: "katex_bridge.js",
});
const throwingKatexFacade = throwingKatexContext.window.__deveWebBridge.get("__deveKatex");
assert.equal(
  throwingKatexFacade.render("x", {}, {}),
  false,
  "KaTeX render facade must fail closed when the engine throws"
);
assert.equal(
  throwingKatexFacade.renderToString("x", {}),
  null,
  "KaTeX renderToString facade must fail closed when the engine throws"
);
assert.throws(
  () => vm.runInNewContext(katexBridgeSource, { window: {} }, {
    filename: "katex_bridge.js",
  }),
  /web bridge registry unavailable before registering katex facade/,
  "KaTeX bridge must fail closed without the registry"
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
  /web bridge registry unavailable before reading deve_code_actions/,
  "init bootstrap must fail closed without the registry"
);
const initUnregisteredWindowContext = {
  window: {
    deve_code_actions: [{ id: "unsafe-unregistered-action" }],
    deve_i18n: {
      locale: "zz-ZZ",
      editor: { noActionsAvailable: "unsafe unregistered copy" },
    },
  },
};
vm.runInNewContext(registrySource, initUnregisteredWindowContext, {
  filename: "web_bridge_registry.js",
});
vm.runInNewContext(initSource, initUnregisteredWindowContext, { filename: "init.js" });
const unregisteredCodeActions =
  initUnregisteredWindowContext.window.__deveWebBridge.get("deve_code_actions");
assert.equal(
  Array.isArray(unregisteredCodeActions) && unregisteredCodeActions.length === 0,
  true,
  "init bootstrap must not adopt unregistered window code actions"
);
assert.equal(
  initUnregisteredWindowContext.window.__deveWebBridge.get("deve_i18n").editor
    .noActionsAvailable,
  "No actions available",
  "init bootstrap must not adopt unregistered window i18n copy"
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
  indexBridgeSource,
  /const registerEditorBridgeGlobal = \(name, value, meta = \{\}\) =>/,
  "index bootstrap must use the bridge registry helper"
);
assert.match(
  indexBridgeSource,
  /web bridge registry unavailable before registering/,
  "index bootstrap must fail closed when the bridge registry is missing"
);
assert.match(
  indexBootstrapSource,
  /registerEditorBridgeGlobal\("__deveEditorBootstrap", editorBootstrapState,\s*\{\s*runtime:\s*"widget_bridge_runtime"/,
  "index editor bootstrap state must be registered through the bridge registry"
);
assert.match(
  indexBootstrapSource,
  /role:\s*"editor-bootstrap-state"/,
  "index editor bootstrap state must declare its bridge role"
);
assert.doesNotMatch(
  indexBridgeSource,
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
    indexBridgeSource,
    new RegExp(`window\\.${name}\\b`),
    `${name} bootstrap state must not be stored directly on window`
  );
}
assert.match(
  indexBootstrapSource,
  /const registerIndexBridgeGlobal = \(name, value, meta = \{\}\) =>/,
  "index boot helpers must use the bridge registry helper"
);
const indexBootHelperMatch = indexBootstrapSource.match(
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
    indexBootstrapSource,
    new RegExp(`registerIndexBridgeGlobal\\("${name}"`),
    `${name} boot helper must be registered through the browser bridge registry`
  );
  assert.doesNotMatch(
    indexBridgeSource,
    new RegExp(`window\\.${name}\\s*=`),
    `${name} boot helper must not be assigned directly in index.html`
  );
  assert.doesNotMatch(
    indexBridgeSource,
    new RegExp(`window\\.${name}\\s*\\(`),
    `${name} boot helper must not be called through scattered window globals in index.html`
  );
}
assert.doesNotMatch(
  indexBridgeSource,
  /window\.__DEVE_DEBUG_OVERLAY__\b/,
  "index boot code must read the debug flag through the bridge facade"
);

assert.doesNotMatch(
  indexBridgeSource,
  /window\.onerror\s*=/,
  "index boot error handling must not assign window.onerror directly"
);

for (const name of indexBridgeNames) {
  assert.match(
    indexBridgeSource,
    new RegExp(`registerEditorBridgeGlobal\\("${name}"`),
    `${name} wrapper must be registered through the browser bridge registry`
  );
  assert.doesNotMatch(
    indexBridgeSource,
    new RegExp(`window\\.${name}\\s*=`),
    `${name} wrapper must not be assigned directly in index.html`
  );
  assert.doesNotMatch(
    indexBridgeSource,
    new RegExp(`window\\.${name}\\s*\\(`),
    `${name} wrapper must not be called through scattered window globals in index.html`
  );
}
assert.doesNotMatch(
  indexBridgeSource,
  /window\.__deveEditorBootstrap\b/,
  "index editor adapter bootstrap must read state through the bridge facade"
);
assert.match(
  indexBridgeSource,
  /bridge\.get\(name\)/,
  "index bootstrap must centralize bridge global reads through registry get()"
);
assert.match(
  indexEditorAdapterSource,
  /bridge\.call\(name, \.\.\.args\)/,
  "index bootstrap must centralize bridge function calls through registry call()"
);
assert.doesNotMatch(
  indexHtmlSource,
  /register(?:Index|Editor)BridgeGlobal|__deveEditorBootstrap|ensureEditorAdapter/,
  "index.html must stay a script loader and not carry bridge implementation logic"
);
assert.doesNotMatch(
  indexHtmlSource,
  /normalizeThemePref|localStorage|data-deve-theme-pref/,
  "index.html must externalize pre-paint theme bootstrap logic"
);
assert.match(
  indexThemeBootstrapSource,
  /data-deve-theme-pref/,
  "theme bootstrap script must own the pre-paint theme marker"
);

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
  chatMathSource,
  /bridge\.get\("__deveKatex"\)/,
  "chat math rendering must read KaTeX through the bridge facade"
);
assert.doesNotMatch(
  chatMathSource,
  /window\.katex\b/,
  "chat math rendering must not read window.katex directly"
);

assert.match(
  renderingBridgeSource,
  /bridge\.get\(name\)/,
  "rendering bridge helper must centralize registry reads"
);
assert.match(
  renderingBridgeSource,
  /getRenderingBridgeGlobal\("__deveKatex"\)/,
  "rendering bridge helper must read the KaTeX facade through the registry"
);
assert.match(
  mathExtensionSource,
  /renderKatex\(this\.content, span/,
  "math extension must render KaTeX through the rendering bridge helper"
);
assert.doesNotMatch(
  mathExtensionSource,
  /window\.katex\b/,
  "math extension must not read window.katex directly"
);
assert.match(
  inlineRendererSource,
  /renderKatex\(mathContent, span/,
  "inline renderer must render KaTeX through the rendering bridge helper"
);
assert.doesNotMatch(
  inlineRendererSource,
  /window\.katex\b/,
  "inline renderer must not read window.katex directly"
);
assert.match(
  outlineKatexSource,
  /"__deveWebBridge"/,
  "outline KaTeX projection must read through the browser bridge registry"
);
assert.match(
  outlineKatexSource,
  /"__deveKatex"/,
  "outline KaTeX projection must use the registered KaTeX facade"
);
assert.match(
  outlineKatexSource,
  /"renderToString"/,
  "outline KaTeX projection must use the facade renderToString method"
);
assert.doesNotMatch(
  outlineKatexSource,
  /JsValue::from_str\("katex"\)/,
  "outline KaTeX projection must not read window.katex directly"
);
assert.match(
  webMainSource,
  /"__deveWebBridge"/,
  "web main boot helpers must read through the browser bridge registry"
);
assert.match(
  webMainSource,
  /Reflect::get\([^;]*"call"/s,
  "web main boot helpers must invoke the registry call facade"
);
assert.match(
  webMainSource,
  /"setBootPanel"/,
  "web main must route setBootPanel by name through the bridge call facade"
);
assert.match(
  webMainSource,
  /"hideBootPanel"/,
  "web main must route hideBootPanel by name through the bridge call facade"
);
assert.doesNotMatch(
  webMainSource,
  /Reflect::get\([^;]*"setBootPanel"/s,
  "web main must not read setBootPanel directly from window"
);
assert.doesNotMatch(
  webMainSource,
  /Reflect::get\([^;]*"hideBootPanel"/s,
  "web main must not read hideBootPanel directly from window"
);
assert.doesNotMatch(
  imageExtensionSource,
  /window\._debug_view\b/,
  "image widget must use the CodeMirror view parameter instead of the debug global"
);
assert.match(
  imageExtensionSource,
  /toDOM\(view\)/,
  "image widget must accept the CodeMirror view parameter"
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
  /window\.deve_code_actions\b/,
  "init bootstrap must not read deve_code_actions directly"
);
assert.doesNotMatch(
  initSource,
  /window\.deve_i18n\s*=/,
  "init bootstrap must not assign deve_i18n directly"
);
assert.doesNotMatch(
  initSource,
  /window\.deve_i18n\b/,
  "init bootstrap must not read deve_i18n directly"
);
assert.doesNotMatch(
  codeMenuSource,
  /window\.deve_code_actions\s*=/,
  "code menu must not initialize the code action registry directly"
);
assert.match(
  widgetBridgeSource,
  /export function getWidgetBridgeGlobal\(name\)/,
  "widget bridge helper must centralize registry reads"
);
assert.match(
  widgetBridgeSource,
  /bridge\.get\(name\)/,
  "widget bridge helper must read browser facades through registry get()"
);
assert.doesNotMatch(
  widgetBridgeSource,
  /(?:window|globalThis)\s*\[\s*name\s*\]/,
  "widget bridge helper must not read facade values through dynamic window/globalThis fields"
);
assert.doesNotMatch(
  widgetBridgeSource,
  /(?:window|globalThis)\s*\[\s*["']deve_(?:code_actions|i18n)["']\s*\]/,
  "widget bridge helper must not read known widget facades through bracket globals"
);
assert.match(
  codeMenuSource,
  /getWidgetBridgeGlobal\("deve_code_actions"\)/,
  "code menu must read actions through the widget bridge facade"
);
assert.doesNotMatch(
  codeMenuSource,
  /window\.deve_code_actions\b/,
  "code menu must not read the code action registry directly from window"
);
assert.doesNotMatch(
  codeMenuSource,
  /(?:window|globalThis)\s*\[\s*["']deve_code_actions["']\s*\]/,
  "code menu must not read the code action registry through bracket globals"
);
assert.match(
  i18nSource,
  /getWidgetBridgeGlobal\("deve_i18n"\)/,
  "i18n copy must read its registry through the widget bridge facade"
);
assert.doesNotMatch(
  i18nSource,
  /window\.deve_i18n\b/,
  "i18n copy must not read the copy registry directly from window"
);
assert.doesNotMatch(
  i18nSource,
  /(?:window|globalThis)\s*\[\s*["']deve_i18n["']\s*\]/,
  "i18n copy must not read the copy registry through bracket globals"
);

const bridgeScriptRev = "20260705-bridge-policy";
function scriptIndex(src) {
  return indexHtmlSource.indexOf(`src="${src}?rev=${bridgeScriptRev}"`);
}

const themeBootstrapScriptIndex = scriptIndex("js/index_theme_bootstrap.js");
const registryScriptIndex = scriptIndex("js/web_bridge_registry.js");
const katexBridgeScriptIndex = scriptIndex("js/katex_bridge.js");
const chatMathBootstrapScriptIndex = scriptIndex("js/chat_math_bootstrap.js");
const chatMathScriptIndex = scriptIndex("js/chat_math.js");
const nativeBackendBridgeScriptIndex = scriptIndex("js/native_backend_bridge.bundle.js");
const indexBootstrapScriptIndex = scriptIndex("js/index_bootstrap.js");
const initScriptIndex = scriptIndex("js/init.js");
const indexEditorAdapterScriptIndex = scriptIndex("js/index_editor_adapter.js");
const lazyEditorBundleIndex = indexEditorAdapterSource.indexOf('import("./editor.bundle.js');
assert.ok(
  themeBootstrapScriptIndex >= 0,
  "index.html must load index_theme_bootstrap.js with the bridge policy rev"
);
assert.ok(
  registryScriptIndex >= 0,
  "index.html must load web_bridge_registry.js with a bridge policy rev"
);
assert.ok(
  katexBridgeScriptIndex >= 0,
  "index.html must load katex_bridge.js with the bridge policy rev"
);
assert.ok(
  chatMathBootstrapScriptIndex >= 0,
  "index.html must load chat_math_bootstrap.js with the bridge policy rev"
);
assert.ok(
  chatMathScriptIndex >= 0,
  "index.html must load chat_math.js with the bridge policy rev"
);
assert.ok(
  nativeBackendBridgeScriptIndex >= 0,
  "index.html must load native_backend_bridge.bundle.js with the bridge policy rev"
);
assert.ok(
  indexBootstrapScriptIndex >= 0,
  "index.html must load index_bootstrap.js with the bridge policy rev"
);
assert.ok(initScriptIndex >= 0, "index.html must load init.js with the bridge policy rev");
assert.ok(
  indexEditorAdapterScriptIndex >= 0,
  "index.html must load index_editor_adapter.js with the bridge policy rev"
);
assert.ok(
  lazyEditorBundleIndex >= 0,
  "index editor adapter must lazy-load the editor bundle"
);
assert.ok(
  themeBootstrapScriptIndex < registryScriptIndex &&
    registryScriptIndex < indexBootstrapScriptIndex &&
    registryScriptIndex < katexBridgeScriptIndex &&
    katexBridgeScriptIndex < chatMathBootstrapScriptIndex &&
    chatMathBootstrapScriptIndex < chatMathScriptIndex &&
    chatMathScriptIndex < nativeBackendBridgeScriptIndex &&
    nativeBackendBridgeScriptIndex < indexBootstrapScriptIndex &&
    indexBootstrapScriptIndex < initScriptIndex &&
    initScriptIndex < indexEditorAdapterScriptIndex,
  "index script order must load theme bootstrap, registry, KaTeX bridge, chat math bootstrap, chat math, native bridge, index bootstrap, init, then lazy editor adapter"
);

console.log("web-bridge-registry-editor-globals: ok");

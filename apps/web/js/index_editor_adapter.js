// index_editor_adapter.js
// Lazy editor adapter binding. Projection/widget bridge only; no authority.

const getEditorBridgeGlobal = (name) => {
  const bridge = window.__deveWebBridge;
  if (!bridge || typeof bridge.get !== "function") {
    throw new Error(`web bridge registry unavailable before reading ${name}`);
  }
  return bridge.get(name);
};

const callEditorBridgeGlobal = (name, ...args) => {
  const bridge = window.__deveWebBridge;
  if (!bridge || typeof bridge.call !== "function") {
    throw new Error(`web bridge registry unavailable before calling ${name}`);
  }
  return bridge.call(name, ...args);
};

const logToOverlay = (...args) => callEditorBridgeGlobal("logToOverlay", ...args);
const hideOverlay = () => callEditorBridgeGlobal("hideOverlay");
const setBootPanel = (...args) => callEditorBridgeGlobal("setBootPanel", ...args);
const queueEditorAction = (...args) => callEditorBridgeGlobal("queueEditorAction", ...args);
const queueEditorMount = (...args) => callEditorBridgeGlobal("queueEditorMount", ...args);
const requestEditorAdapter = () => {
  const ensureEditorAdapter = getEditorBridgeGlobal("ensureEditorAdapter");
  if (typeof ensureEditorAdapter === "function") {
    return ensureEditorAdapter();
  }
  return false;
};

logToOverlay("Module Script Started. Adapter will lazy-load on demand.");

const registerEditorBridgeGlobal = (name, value, meta = {}) => {
  const bridge = window.__deveWebBridge;
  if (!bridge || typeof bridge.register !== "function") {
    throw new Error(`web bridge registry unavailable before registering ${name}`);
  }
  return bridge.register(name, value, {
    runtime: "render_projection_runtime",
    source: "index-editor-adapter",
    authority: "none",
    ...meta,
  });
};

const editorBootstrapState = getEditorBridgeGlobal("__deveEditorBootstrap");
if (!editorBootstrapState) {
  throw new Error("editor bootstrap state unavailable before loading adapter");
}

const attachEditorAdapter = (module) => {
  const {
    initCodeMirror,
    applyRemoteContent,
    applyRemoteOp,
    applyRemoteOpsBatch,
    getEditorContent,
    syncEditorStateToRust,
    scrollGlobal,
    setReadOnly,
    setReadOnlyForHost,
    destroyEditor,
    updateGutterDiff,
    getEditorSelection,
  } = module;

  const rawApplyRemoteContent = (text) => applyRemoteContent(text) === true;
  const rawApplyRemoteOp = (opJson) => {
    return applyRemoteOp(opJson) === true;
  };
  const rawApplyRemoteOpsBatch =
    applyRemoteOpsBatch ||
    ((opsJson) => {
      try {
        const ops = JSON.parse(opsJson);
        if (!Array.isArray(ops)) return false;
        for (const op of ops) {
        if (!op || (!op.Insert && !op.Delete)) return false;
        if (!rawApplyRemoteOp(JSON.stringify(op))) return false;
      }
      return true;
      } catch (e) {
        console.error("applyRemoteOpsBatch Fallback Error:", e);
        return false;
      }
    });
  const rawSetReadOnly = (readOnly) => {
    return setReadOnly(readOnly) === true;
  };
  const rawSetReadOnlyForHost = (expectedHost, readOnly) => {
    return setReadOnlyForHost(expectedHost, readOnly) === true;
  };
  const rawDestroyEditor = (expectedHost) => {
    return destroyEditor(expectedHost) === true;
  };
  const replayEditorAction = (action) => {
    switch (action.kind) {
      case "content":
        if (!rawApplyRemoteContent(action.payload)) {
          console.warn("Queued editor content replay failed");
        }
        break;
      case "op":
        rawApplyRemoteOp(action.payload);
        break;
      case "opsBatch":
        if (rawApplyRemoteOpsBatch(action.payload) !== true) {
          console.warn("Queued editor ops batch replay failed");
        }
        break;
      case "readOnly":
        rawSetReadOnly(action.payload);
        break;
      case "readOnlyForHost":
        rawSetReadOnlyForHost(
          action.payload.expectedHost,
          action.payload.readOnly,
        );
        break;
      default:
        break;
    }
  };

  registerEditorBridgeGlobal("applyRemoteContent", (text) => {
    if (!editorBootstrapState.editorBridgeReady) {
      requestEditorAdapter();
      return false;
    }
    return rawApplyRemoteContent(text);
  }, { role: "wasm-editor-snapshot" });

  registerEditorBridgeGlobal("applyRemoteOp", (opJson) => {
    if (!editorBootstrapState.editorBridgeReady) {
      requestEditorAdapter();
      return false;
    }
    return rawApplyRemoteOp(opJson);
  }, { role: "wasm-editor-op" });

  registerEditorBridgeGlobal("applyRemoteOpsBatch", (opsJson) => {
    if (!editorBootstrapState.editorBridgeReady) {
      return false;
    }
    return rawApplyRemoteOpsBatch(opsJson) === true;
  }, { role: "wasm-editor-op-batch" });

  registerEditorBridgeGlobal("getEditorContent", getEditorContent, {
    role: "wasm-editor-query",
  });

  registerEditorBridgeGlobal("syncEditorStateToRust", () => {
    if (!editorBootstrapState.editorBridgeReady || typeof syncEditorStateToRust !== "function") {
      return false;
    }
    syncEditorStateToRust();
    return true;
  }, { role: "wasm-editor-sync" });

  registerEditorBridgeGlobal("scrollGlobal", scrollGlobal, {
    runtime: "widget_bridge_runtime",
    role: "wasm-editor-navigation",
  });

  registerEditorBridgeGlobal("setReadOnly", (readOnly) => {
    if (!editorBootstrapState.editorBridgeReady) {
      queueEditorAction("readOnly", !!readOnly);
      return false;
    }
    return rawSetReadOnly(readOnly);
  }, { role: "wasm-editor-readonly" });

  registerEditorBridgeGlobal("setReadOnlyForHost", (expectedHost, readOnly) => {
    if (!editorBootstrapState.ownsHost(expectedHost)) return false;
    if (!editorBootstrapState.editorBridgeReady) {
      queueEditorAction("readOnlyForHost", { expectedHost, readOnly: !!readOnly });
      return false;
    }
    return rawSetReadOnlyForHost(expectedHost, readOnly);
  }, { role: "wasm-editor-owner-readonly" });

  registerEditorBridgeGlobal("destroyEditor", (expectedHost) => {
    if (!editorBootstrapState.ownsHost(expectedHost)) return false;
    try {
      return rawDestroyEditor(expectedHost) === true;
    } finally {
      editorBootstrapState.resetBridge(expectedHost);
    }
  }, { role: "wasm-editor-lifecycle" });

  registerEditorBridgeGlobal("updateGutterDiff", updateGutterDiff || (() => {}), {
    runtime: "widget_bridge_runtime",
    role: "wasm-editor-diff-projection",
  });

  registerEditorBridgeGlobal("getEditorSelection", getEditorSelection || (() => "null"), {
    role: "wasm-editor-selection",
  });

  if (typeof initCodeMirror !== "function") {
    throw new Error("initCodeMirror is not a function in exported module!");
  }

  logToOverlay("Adapter imported successfully.");

  editorBootstrapState.realInit = (element, onUpdate, onReady) => {
    if (element?.isConnected === false) return false;
    logToOverlay("Initializing Editor View");
    hideOverlay();

    try {
      initCodeMirror(element, onUpdate);
      editorBootstrapState.activeHost = element;
      editorBootstrapState.editorBridgeReady = true;
      if (editorBootstrapState.pendingEditorActions.length > 0) {
        const pending = editorBootstrapState.takePendingActions();
        logToOverlay(
          `Replaying ${pending.length} queued editor actions...`,
        );
        pending.forEach(replayEditorAction);
      }
      rawSetReadOnly(true);
      syncEditorStateToRust?.();
      if (typeof onReady !== "function") {
        throw new TypeError("CodeMirror readiness callback is required");
      }
      onReady();
      return true;
    } catch (e) {
      try {
        rawSetReadOnly(true);
      } catch (_) {
        // Keep the original initialization error as the visible failure.
      }
      try {
        rawDestroyEditor(element);
      } catch (_) {
        // The bridge remains unready even if best-effort cleanup also fails.
      }
      editorBootstrapState.failMount();
      logToOverlay("Init call failed: " + e.message, true);
      throw e;
    }
  };

  registerEditorBridgeGlobal("setupCodeMirror", (element, onUpdate, onReady) => {
    if (element?.isConnected === false) return false;
    logToOverlay("Rust called setupCodeMirror");
    if (editorBootstrapState.realInit) {
      return editorBootstrapState.realInit(element, onUpdate, onReady) === true;
    }
    if (queueEditorMount(element, onUpdate, onReady) === false) return false;
    requestEditorAdapter();
    return true;
  }, { role: "wasm-editor-mount" });

  editorBootstrapState.cmLoaded = true;

  const queuedMount = editorBootstrapState.takeLatestMount();
  if (queuedMount) {
    logToOverlay(
      `Flushing latest editor mount from ${queuedMount.queuedCount} queued item(s)...`,
    );
    editorBootstrapState.realInit(
      queuedMount.latestMount.element,
      queuedMount.latestMount.onUpdate,
      queuedMount.latestMount.onReady,
    );
  }
};

registerEditorBridgeGlobal("ensureEditorAdapter", () => {
  if (editorBootstrapState.cmLoaded) {
    return Promise.resolve();
  }
  if (editorBootstrapState.adapterLoading) {
    return editorBootstrapState.adapterLoading;
  }

  setBootPanel(
    "Loading Editor Adapter",
    "Editor requested CodeMirror. Loading adapter bundle now.",
    "success",
  );
  logToOverlay("Lazy-loading editor adapter bundle...");

  editorBootstrapState.adapterLoading = import("./editor.bundle.js?rev=20260721-active-companion-preview")
    .then((module) => {
      attachEditorAdapter(module);
    })
    .catch((e) => {
      console.error("Failed to load editor adapter:", e);
      logToOverlay(
        "Failed to load editor adapter: " + e.message,
        true,
      );
      if (e.stack) logToOverlay(e.stack, true);
      throw e;
    })
    .finally(() => {
      if (!editorBootstrapState.cmLoaded) {
        editorBootstrapState.adapterLoading = null;
      }
    });

  return editorBootstrapState.adapterLoading;
}, { runtime: "widget_bridge_runtime", role: "editor-adapter-loader" });

window.addEventListener("TrunkApplicationStarted", () => {
  if (getEditorBridgeGlobal("__DEVE_DEBUG_OVERLAY__") === true) {
    console.log("[Overlay]", "TrunkApplicationStarted event received.");
  }
}, { once: true });

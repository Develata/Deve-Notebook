// index_bootstrap.js
// Early browser bridge bootstrap for boot UI and editor queue stubs.

(function () {
  const registerIndexBridgeGlobal = (name, value, meta = {}) => {
    const bridge = window.__deveWebBridge;
    if (!bridge || typeof bridge.register !== "function") {
      throw new Error(`web bridge registry unavailable before registering ${name}`);
    }
    return bridge.register(name, value, {
      runtime: "widget_bridge_runtime",
      source: "index-boot-bootstrap",
      authority: "none",
      ...meta,
    });
  };

  const getIndexBridgeGlobal = (name) => {
    const bridge = window.__deveWebBridge;
    if (!bridge || typeof bridge.get !== "function") {
      throw new Error(`web bridge registry unavailable before reading ${name}`);
    }
    return bridge.get(name);
  };

  const isDebugOverlayEnabled = () =>
    getIndexBridgeGlobal("__DEVE_DEBUG_OVERLAY__") === true;

  registerIndexBridgeGlobal("__DEVE_DEBUG_OVERLAY__", false, {
    role: "boot-debug-flag",
  });

  const showOverlay = registerIndexBridgeGlobal("showOverlay", () => {
    const el = document.getElementById("loading-overlay");
    if (!el) return;
    el.classList.remove("hidden");
    el.style.display = "flex";
  }, { role: "boot-overlay-show" });

  const setBootPanel = registerIndexBridgeGlobal("setBootPanel", (title, detail, tone = "info") => {
    const panel = document.getElementById("boot-panel");
    const titleEl = document.getElementById("boot-panel-title");
    const detailEl = document.getElementById("boot-panel-detail");
    if (!panel || !titleEl || !detailEl) return;
    if (tone !== "error") {
      panel.style.display = "none";
      return;
    }
    panel.style.display = "block";
    panel.className =
      "fixed top-4 left-4 z-[var(--z-toast)] max-w-lg rounded-xl border border-red-400 bg-red-50 px-4 py-3 text-red-900 shadow-lg";
    titleEl.textContent = title;
    detailEl.textContent = detail;
  }, { role: "boot-error-panel-set" });

  registerIndexBridgeGlobal("hideBootPanel", () => {
    const panel = document.getElementById("boot-panel");
    if (!panel) return;
    panel.style.display = "none";
  }, { role: "boot-error-panel-hide" });

  registerIndexBridgeGlobal("hideOverlay", () => {
    const el = document.getElementById("loading-overlay");
    if (!el) return;
    el.classList.add("hidden");
    el.style.display = "none";
  }, { role: "boot-overlay-hide" });

  const logToOverlay = registerIndexBridgeGlobal("logToOverlay", (msg, isError = false) => {
    const el = document.getElementById("loading-overlay");
    const debugOverlay = isDebugOverlayEnabled();
    if (el && (isError || debugOverlay)) {
      if (isError) showOverlay();
      const line = document.createElement("div");
      if (isError) line.style.color = "red";
      line.innerText = msg;
      el.appendChild(line);
    }
    if (isError) {
      console.error("[Overlay]", msg);
      setBootPanel("Boot Error", msg, "error");
    } else if (debugOverlay) {
      console.log("[Overlay]", msg);
    }
  }, { role: "boot-overlay-log" });

  window.addEventListener("error", function (event) {
    const msg = event.message || String(event.error || "unknown error");
    const line = event.lineno ?? 0;
    const col = event.colno ?? 0;
    logToOverlay(`Global Error: ${msg} @ ${line}:${col}`, true);
  });

  window.addEventListener("unhandledrejection", function (event) {
    logToOverlay(`Unhandled Rejection: ${event.reason}`, true);
  });

  const registerEditorBridgeGlobal = (name, value, meta = {}) => {
    const bridge = window.__deveWebBridge;
    if (!bridge || typeof bridge.register !== "function") {
      throw new Error(`web bridge registry unavailable before registering ${name}`);
    }
    return bridge.register(name, value, {
      runtime: "render_projection_runtime",
      source: "index-editor-bootstrap",
      authority: "none",
      ...meta,
    });
  };

  const getEditorBridgeGlobal = (name) => {
    const bridge = window.__deveWebBridge;
    if (!bridge || typeof bridge.get !== "function") {
      throw new Error(`web bridge registry unavailable before reading ${name}`);
    }
    return bridge.get(name);
  };

  const requestEditorAdapter = () => {
    const ensureEditorAdapter = getEditorBridgeGlobal("ensureEditorAdapter");
    if (typeof ensureEditorAdapter === "function") {
      return ensureEditorAdapter();
    }
    return false;
  };

  const editorBootstrapState = {
    editorQueue: [],
    pendingEditorActions: [],
    cmLoaded: false,
    editorBridgeReady: false,
    realInit: null,
    adapterLoading: null,
    queueAction(kind, payload) {
      this.pendingEditorActions.push({ kind, payload });
      if (this.pendingEditorActions.length > 256) {
        this.pendingEditorActions.shift();
      }
    },
    queueMount(element, onUpdate) {
      this.editorQueue = [{ element, onUpdate }];
    },
    resetBridge() {
      this.editorBridgeReady = false;
      this.pendingEditorActions = [];
    },
    takePendingActions() {
      const pending = this.pendingEditorActions.slice();
      this.pendingEditorActions = [];
      return pending;
    },
    takeLatestMount() {
      if (this.editorQueue.length === 0) return null;
      const pendingMounts = this.editorQueue.slice();
      this.editorQueue = [];
      return {
        latestMount: pendingMounts[pendingMounts.length - 1],
        queuedCount: pendingMounts.length,
      };
    },
  };

  registerEditorBridgeGlobal("__deveEditorBootstrap", editorBootstrapState, {
    runtime: "widget_bridge_runtime",
    role: "editor-bootstrap-state",
  });

  const queueEditorAction = registerEditorBridgeGlobal("queueEditorAction", (kind, payload) => {
    editorBootstrapState.queueAction(kind, payload);
  }, { role: "editor-bootstrap-queue" });

  const queueEditorMount = registerEditorBridgeGlobal("queueEditorMount", (element, onUpdate) => {
    editorBootstrapState.queueMount(element, onUpdate);
  }, { role: "editor-bootstrap-mount-queue" });

  registerEditorBridgeGlobal("setupCodeMirror", function (element, onUpdate) {
    logToOverlay("Rust called setupCodeMirror");
    if (editorBootstrapState.cmLoaded && editorBootstrapState.realInit) {
      return editorBootstrapState.realInit(element, onUpdate) === true;
    } else {
      queueEditorMount(element, onUpdate);
      requestEditorAdapter();
      return true;
    }
  }, { role: "wasm-editor-mount" });

  registerEditorBridgeGlobal("applyRemoteContent", () => {
    requestEditorAdapter();
    return false;
  }, { role: "wasm-editor-snapshot" });

  registerEditorBridgeGlobal("applyRemoteOp", () => {
    requestEditorAdapter();
    return false;
  }, { role: "wasm-editor-op" });

  registerEditorBridgeGlobal("applyRemoteOpsBatch", () => false, {
    role: "wasm-editor-op-batch",
  });

  registerEditorBridgeGlobal("getEditorContent", () => null, {
    role: "wasm-editor-query",
  });

  registerEditorBridgeGlobal("syncEditorStateToRust", () => false, {
    role: "wasm-editor-sync",
  });

  registerEditorBridgeGlobal("scrollGlobal", () => {}, {
    runtime: "widget_bridge_runtime",
    role: "wasm-editor-navigation",
  });

  registerEditorBridgeGlobal("updateGutterDiff", () => {}, {
    runtime: "widget_bridge_runtime",
    role: "wasm-editor-diff-projection",
  });

  registerEditorBridgeGlobal("getEditorSelection", () => "null", {
    role: "wasm-editor-selection",
  });

  registerEditorBridgeGlobal("setReadOnly", (readOnly) => {
    queueEditorAction("readOnly", !!readOnly);
    return false;
  }, { role: "wasm-editor-readonly" });

  registerEditorBridgeGlobal("destroyEditor", () => {
    editorBootstrapState.resetBridge();
    return true;
  }, { role: "wasm-editor-lifecycle" });

  const ensureMobileEditorAdapter = () => {
    requestEditorAdapter();
    return false;
  };

  registerEditorBridgeGlobal("mobileInsertText", () => ensureMobileEditorAdapter(), {
    runtime: "widget_bridge_runtime",
    role: "wasm-editor-mobile-input",
  });

  registerEditorBridgeGlobal("mobileWrapSelection", () => ensureMobileEditorAdapter(), {
    runtime: "widget_bridge_runtime",
    role: "wasm-editor-mobile-input",
  });

  registerEditorBridgeGlobal("mobileUndo", () => ensureMobileEditorAdapter(), {
    runtime: "widget_bridge_runtime",
    role: "wasm-editor-history",
  });

  registerEditorBridgeGlobal("mobileRedo", () => ensureMobileEditorAdapter(), {
    runtime: "widget_bridge_runtime",
    role: "wasm-editor-history",
  });
})();

// apps/web/js/web_bridge_registry.js
// Central compatibility registry for browser globals exposed to Rust/WASM.

(function () {
  const existing = window.__deveWebBridge;
  if (
    existing &&
    typeof existing.register === "function" &&
    typeof existing.registerFallback === "function" &&
    typeof existing.describe === "function" &&
    typeof existing.get === "function" &&
    typeof existing.call === "function"
  ) {
    return;
  }

  const entries = new Map();

  function register(name, value, meta = {}) {
    entries.set(name, { value, meta });
    window[name] = value;
    return value;
  }

  function registerFallback(name, value, meta = {}) {
    if (typeof window[name] !== "undefined") {
      entries.set(name, { value: window[name], meta: { ...meta, fallbackSkipped: true } });
      return window[name];
    }
    return register(name, value, meta);
  }

  function get(name) {
    const entry = entries.get(name);
    if (!entry) {
      return undefined;
    }
    return entry.value;
  }

  function call(name, ...args) {
    const value = get(name);
    if (typeof value !== "function") {
      throw new Error(`web bridge global ${name} is not callable`);
    }
    return value(...args);
  }

  function describe() {
    return Array.from(entries.entries()).map(([name, entry]) => ({
      name,
      meta: entry.meta,
    }));
  }

  if (existing && typeof existing.describe === "function") {
    for (const entry of existing.describe()) {
      if (entry && entry.name && typeof window[entry.name] !== "undefined") {
        entries.set(entry.name, {
          value: window[entry.name],
          meta: { ...(entry.meta || {}), adopted: true },
        });
      }
    }
  }

  window.__deveWebBridge = {
    register,
    registerFallback,
    get,
    call,
    describe,
  };
})();

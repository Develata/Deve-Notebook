// apps/web/js/web_bridge_registry.js
// Central compatibility registry for browser globals exposed to Rust/WASM.

(function () {
  const policyVersion = "projection-only-v1";
  const existing = window.__deveWebBridge;
  const entries = new Map();
  const allowedRuntimes = new Set([
    "render_projection_runtime",
    "widget_bridge_runtime",
    "native_shell_mode_runtime",
  ]);
  const authoritySequences = [
    ["pending"],
    ["ack"],
    ["reject"],
    ["write", "success"],
    ["source", "control"],
    ["ledger"],
    ["staging"],
    ["commit", "anchor"],
    ["git", "mirror"],
    ["pending", "fs"],
    ["pending", "fs", "ops"],
    ["backup"],
    ["remote", "projection"],
  ];
  const singleAuthorityTokens = authoritySequences
    .filter((sequence) => sequence.length === 1)
    .map((sequence) => sequence[0]);
  const collapsedAuthoritySequences = authoritySequences
    .filter((sequence) => sequence.length > 1)
    .map((sequence) => sequence.join(""));

  function semanticTokens(value) {
    return String(value || "")
      .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
      .toLowerCase()
      .split(/[^a-z0-9]+/)
      .filter(Boolean);
  }

  function containsSequence(tokens, sequence) {
    for (let i = 0; i <= tokens.length - sequence.length; i += 1) {
      if (sequence.every((token, offset) => tokens[i + offset] === token)) {
        return true;
      }
    }
    return false;
  }

  function containsCollapsedSingleAuthorityToken(tokens) {
    return tokens.some((token) =>
      singleAuthorityTokens.some((authorityToken) => {
        if (token === authorityToken || token.startsWith(authorityToken)) {
          return true;
        }
        return authorityToken.length > 3 && token.endsWith(authorityToken);
      })
    );
  }

  function containsAuthoritySemantic(value) {
    const tokens = semanticTokens(value);
    if (authoritySequences.some((sequence) => containsSequence(tokens, sequence))) {
      return true;
    }
    if (containsCollapsedSingleAuthorityToken(tokens)) {
      return true;
    }
    const collapsedTokens = tokens.join("");
    return collapsedAuthoritySequences.some((sequence) => collapsedTokens.includes(sequence));
  }

  function normalizeMeta(name, meta = {}) {
    const normalized = { ...meta };
    if (!allowedRuntimes.has(normalized.runtime)) {
      throw new Error(`web bridge global ${name} has unsupported runtime`);
    }
    if (normalized.authority !== "none") {
      throw new Error(`web bridge global ${name} must declare authority none`);
    }
    if (
      containsAuthoritySemantic(name) ||
      containsAuthoritySemantic(normalized.source) ||
      containsAuthoritySemantic(normalized.role)
    ) {
      throw new Error(`web bridge global ${name} metadata must stay projection-only`);
    }
    return normalized;
  }

  function register(name, value, meta = {}) {
    const normalizedMeta = normalizeMeta(name, meta);
    entries.set(name, { value, meta: normalizedMeta });
    window[name] = value;
    return value;
  }

  function registerFallback(name, value, meta = {}) {
    const normalizedMeta = normalizeMeta(name, meta);
    const existingEntry = entries.get(name);
    if (existingEntry) {
      entries.set(name, {
        value: existingEntry.value,
        meta: { ...normalizedMeta, fallbackSkipped: true },
      });
      window[name] = existingEntry.value;
      return existingEntry.value;
    }
    return register(name, value, normalizedMeta);
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

  if (
    existing &&
    typeof existing.describe === "function" &&
    typeof existing.get === "function"
  ) {
    for (const entry of existing.describe()) {
      if (entry && entry.name) {
        const value = existing.get(entry.name);
        if (typeof value === "undefined") {
          continue;
        }
        const adoptedMeta = normalizeMeta(entry.name, { ...(entry.meta || {}), adopted: true });
        entries.set(entry.name, {
          value,
          meta: adoptedMeta,
        });
        window[entry.name] = value;
      }
    }
  }

  window.__deveWebBridge = {
    policyVersion,
    register,
    registerFallback,
    get,
    call,
    describe,
  };
})();

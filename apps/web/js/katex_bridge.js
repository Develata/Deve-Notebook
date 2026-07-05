// apps/web/js/katex_bridge.js
// Central KaTeX facade for rendering-only browser bridge consumers.

(function () {
  const bridge = window.__deveWebBridge;
  if (!bridge || typeof bridge.register !== "function") {
    throw new Error("web bridge registry unavailable before registering katex facade");
  }

  const katexFacade = {
    available() {
      return typeof window.katex?.render === "function";
    },
    render(content, element, options = {}) {
      const katex = window.katex;
      if (!katex || typeof katex.render !== "function") {
        return false;
      }
      katex.render(content, element, options);
      return true;
    },
  };

  bridge.register("__deveKatex", katexFacade, {
    runtime: "render_projection_runtime",
    source: "katex_bridge",
    authority: "none",
    role: "katex-render-facade",
  });
})();

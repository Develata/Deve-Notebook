// apps/web/js/chat_math_bootstrap.js
// Registers the chat math fallback through the rendering client bridge.

(function () {
  const bridge = window.__deveWebBridge;
  if (!bridge || typeof bridge.registerFallback !== "function") {
    throw new Error("web bridge registry unavailable before registering renderChatMath");
  }

  bridge.registerFallback("renderChatMath", () => false, {
    runtime: "render_projection_runtime",
    source: "chat_math_bootstrap",
    authority: "none",
    role: "chat-math-fallback",
  });
})();

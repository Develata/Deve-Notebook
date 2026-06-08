// apps/web/js/chat_math_bootstrap.js
// Registers the chat math fallback through the rendering client bridge.

(function () {
  const bridge = window.__deveWebBridge;
  if (!bridge || typeof bridge.registerFallback !== "function") {
    window.renderChatMath = window.renderChatMath || (() => false);
    return;
  }

  bridge.registerFallback("renderChatMath", () => false, {
    runtime: "rendering_client",
    boundary: "object-plane-adapter",
  });
})();

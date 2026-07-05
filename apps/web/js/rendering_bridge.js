// apps/web/js/rendering_bridge.js
// Shared helpers for rendering-only modules that need browser bridge facades.

function browserWindow() {
  return typeof window !== "undefined" ? window : undefined;
}

export function getRenderingBridgeGlobal(name) {
  const bridge = browserWindow()?.__deveWebBridge;
  if (!bridge || typeof bridge.get !== "function") {
    return undefined;
  }
  return bridge.get(name);
}

function setFallbackText(element, fallbackText) {
  if (element && typeof fallbackText === "string") {
    element.textContent = fallbackText;
  }
}

export function renderKatex(content, element, options = {}, fallbackText = "") {
  const katex = getRenderingBridgeGlobal("__deveKatex");
  if (!katex || typeof katex.render !== "function") {
    setFallbackText(element, fallbackText);
    return false;
  }

  try {
    const rendered = katex.render(content, element, options);
    if (rendered === false) {
      setFallbackText(element, fallbackText);
      return false;
    }
    return true;
  } catch (_err) {
    setFallbackText(element, fallbackText);
    return false;
  }
}

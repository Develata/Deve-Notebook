// apps/web/js/widget_bridge.js
// Shared helpers for widget-only modules that need browser bridge facades.

function browserWindow() {
  return typeof window !== "undefined" ? window : undefined;
}

export function getWidgetBridgeGlobal(name) {
  const bridge = browserWindow()?.__deveWebBridge;
  if (!bridge || typeof bridge.get !== "function") {
    return undefined;
  }
  return bridge.get(name);
}

const DELIVERY_GATE_KEY = "__deveLifecycleWsDeliveryGate";

// Smoke-only outbound transport gate. It is installed before page load through
// CDP and never ships in the product bundle. Buffered frames belong to the old
// transport generation and can only be discarded, never replayed by the gate;
// the product must replay its own pending overlay on the replacement session.
export function installWebSocketDeliveryGateInPage() {
  const gateKey = "__deveLifecycleWsDeliveryGate";
  if (globalThis[gateKey]) return;

  const nativeSend = WebSocket.prototype.send;
  let paused = false;
  let queue = [];
  let intercepted = 0;

  WebSocket.prototype.send = function gatedSend(data) {
    if (!paused) return nativeSend.call(this, data);
    intercepted += 1;
    queue.push({ socket: this, data });
    return undefined;
  };

  globalThis[gateKey] = Object.freeze({
    pause() {
      paused = true;
      intercepted = 0;
      return { paused, pending: queue.length, intercepted };
    },
    discard() {
      paused = false;
      const released = queue.length;
      queue = [];
      return { paused, released, delivered: false };
    },
    snapshot() {
      return { paused, pending: queue.length, intercepted };
    },
  });
}

export function webSocketDeliveryGateSource() {
  return `(${installWebSocketDeliveryGateInPage.toString()})();`;
}

export async function installWebSocketDeliveryGate(page) {
  await page.send("Page.enable");
  const result = await page.send("Page.addScriptToEvaluateOnNewDocument", {
    source: webSocketDeliveryGateSource(),
  });
  return result.identifier;
}

async function controlWebSocketDeliveryGate(page, action) {
  const result = await page.call((gateKey, gateAction) => {
    const gate = globalThis[gateKey];
    if (!gate || typeof gate[gateAction] !== "function") return null;
    return gate[gateAction]();
  }, DELIVERY_GATE_KEY, action);
  if (!result) throw new Error(`WebSocket delivery gate action unavailable: ${action}`);
  return result;
}

export function pauseWebSocketDelivery(page) {
  return controlWebSocketDeliveryGate(page, "pause");
}

export function discardAndResumeWebSocketDelivery(page) {
  return controlWebSocketDeliveryGate(page, "discard");
}

export function inspectWebSocketDeliveryGate(page) {
  return controlWebSocketDeliveryGate(page, "snapshot");
}

import assert from "node:assert/strict";
import test from "node:test";

import {
  installWebSocketDeliveryGate,
  installWebSocketDeliveryGateInPage,
} from "./lib/websocket-delivery-gate.mjs";

const gateKey = "__deveLifecycleWsDeliveryGate";

function withFakeWebSocket(run) {
  const originalWebSocket = globalThis.WebSocket;
  class FakeWebSocket {
    constructor() {
      this.sent = [];
    }

    send(data) {
      this.sent.push(data);
    }
  }
  globalThis.WebSocket = FakeWebSocket;
  delete globalThis[gateKey];
  try {
    installWebSocketDeliveryGateInPage();
    run(FakeWebSocket);
  } finally {
    delete globalThis[gateKey];
    globalThis.WebSocket = originalWebSocket;
  }
}

test("delivery gate leaves normal WebSocket sends untouched", () => {
  withFakeWebSocket((FakeWebSocket) => {
    const socket = new FakeWebSocket();
    socket.send("handshake");
    assert.deepEqual(socket.sent, ["handshake"]);
  });
});

test("delivery gate buffers outbound frames while paused", () => {
  withFakeWebSocket((FakeWebSocket) => {
    const socket = new FakeWebSocket();
    assert.deepEqual(globalThis[gateKey].pause(), {
      paused: true,
      pending: 0,
      intercepted: 0,
    });

    socket.send("delta-1");
    socket.send(new Uint8Array([2]));
    assert.deepEqual(socket.sent, []);
    assert.deepEqual(globalThis[gateKey].snapshot(), {
      paused: true,
      pending: 2,
      intercepted: 2,
    });
  });
});

test("discard resumes sends without replaying stale-generation frames", () => {
  withFakeWebSocket((FakeWebSocket) => {
    const socket = new FakeWebSocket();
    globalThis[gateKey].pause();
    socket.send("stale-delta");

    assert.deepEqual(globalThis[gateKey].discard(), {
      paused: false,
      released: 1,
      delivered: false,
    });
    assert.deepEqual(socket.sent, []);

    socket.send("replacement-generation-replay");
    assert.deepEqual(socket.sent, ["replacement-generation-replay"]);
  });
});

test("CDP installer registers the gate before the next document", async () => {
  const calls = [];
  const page = {
    async send(method, params) {
      calls.push({ method, params });
      return method === "Page.addScriptToEvaluateOnNewDocument"
        ? { identifier: "gate-script" }
        : {};
    },
  };

  assert.equal(await installWebSocketDeliveryGate(page), "gate-script");
  assert.equal(calls[0].method, "Page.enable");
  assert.equal(calls[1].method, "Page.addScriptToEvaluateOnNewDocument");
  assert.match(calls[1].params.source, /__deveLifecycleWsDeliveryGate/);
  assert.match(calls[1].params.source, /WebSocket\.prototype\.send/);
});

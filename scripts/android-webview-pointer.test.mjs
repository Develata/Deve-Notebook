import assert from "node:assert/strict";
import test from "node:test";

import { clickWebViewPoint, tapWebViewPoint } from "./lib/android-webview-pointer.mjs";

test("WebView point click dispatches one complete primary-button gesture", async () => {
  const sent = [];
  const page = {
    async send(method, params) {
      sent.push({ method, params });
    },
  };

  await clickWebViewPoint(page, { x: 24, y: 32 });

  assert.deepEqual(sent, [
    {
      method: "Input.dispatchMouseEvent",
      params: { type: "mouseMoved", x: 24, y: 32, button: "none", buttons: 0 },
    },
    {
      method: "Input.dispatchMouseEvent",
      params: {
        type: "mousePressed",
        x: 24,
        y: 32,
        button: "left",
        buttons: 1,
        clickCount: 1,
        force: 0.5,
      },
    },
    {
      method: "Input.dispatchMouseEvent",
      params: {
        type: "mouseReleased",
        x: 24,
        y: 32,
        button: "left",
        buttons: 0,
        clickCount: 1,
      },
    },
  ]);
});

test("WebView point click revalidates after move and stops on every failed stage", async () => {
  const beforePressSent = [];
  await assert.rejects(
    clickWebViewPoint(
      { send: async (_method, params) => beforePressSent.push(params.type) },
      { x: 24, y: 32 },
      { beforePress: async () => { throw new Error("target moved"); } },
    ),
    /target moved/,
  );
  assert.deepEqual(beforePressSent, ["mouseMoved"]);

  for (const failedType of ["mouseMoved", "mousePressed", "mouseReleased"]) {
    const sent = [];
    const page = {
      async send(_method, params) {
        sent.push(params.type);
        if (params.type === failedType) throw new Error(`failed ${failedType}`);
      },
    };
    await assert.rejects(
      clickWebViewPoint(page, { x: 24, y: 32 }),
      new RegExp(`failed ${failedType}`),
    );
    const failedIndex = ["mouseMoved", "mousePressed", "mouseReleased"].indexOf(failedType);
    assert.deepEqual(sent, ["mouseMoved", "mousePressed", "mouseReleased"].slice(0, failedIndex + 1));
  }
});

test("WebView point click presses a canonical point reacquired after hover", async () => {
  const sent = [];
  const page = {
    async send(_method, params) {
      sent.push({ type: params.type, x: params.x, y: params.y });
    },
  };

  await clickWebViewPoint(
    page,
    { x: 24, y: 32 },
    { beforePress: async () => ({ x: 124, y: 132 }) },
  );

  assert.deepEqual(sent, [
    { type: "mouseMoved", x: 24, y: 32 },
    { type: "mousePressed", x: 124, y: 132 },
    { type: "mouseReleased", x: 124, y: 132 },
  ]);
});

test("WebView point click rejects invalid initial and replacement points before press", async () => {
  const sent = [];
  const page = { send: async (_method, params) => sent.push(params.type) };
  await assert.rejects(
    clickWebViewPoint(page, { x: Number.NaN, y: 32 }),
    /invalid initial point/,
  );
  assert.deepEqual(sent, []);

  await assert.rejects(
    clickWebViewPoint(
      page,
      { x: 24, y: 32 },
      { beforePress: async () => ({ x: Number.POSITIVE_INFINITY, y: 132 }) },
    ),
    /invalid point/,
  );
  assert.deepEqual(sent, ["mouseMoved"]);
});

test("WebView point tap holds one bounded 50 ms contact before touchEnd", async () => {
  const sent = [];
  const contactWaits = [];
  const page = {
    async send(method, params) {
      sent.push({ method, params });
    },
  };

  await tapWebViewPoint(
    page,
    { x: 24, y: 32 },
    { waitForContact: async (durationMs) => { contactWaits.push(durationMs); } },
  );

  assert.deepEqual(contactWaits, [50]);
  assert.deepEqual(sent, [
    {
      method: "Input.dispatchTouchEvent",
      params: {
        type: "touchStart",
        touchPoints: [{ x: 24, y: 32, radiusX: 1, radiusY: 1, force: 1, id: 0 }],
      },
    },
    {
      method: "Input.dispatchTouchEvent",
      params: { type: "touchEnd", touchPoints: [] },
    },
  ]);
});

test("WebView point tap revalidates before contact and stops on every failed stage", async () => {
  const beforeContactSent = [];
  await assert.rejects(
    tapWebViewPoint(
      { send: async (_method, params) => beforeContactSent.push(params.type) },
      { x: 24, y: 32 },
      { beforeContact: async () => { throw new Error("target moved"); } },
    ),
    /target moved/,
  );
  assert.deepEqual(beforeContactSent, []);

  for (const failedType of ["touchStart", "touchEnd"]) {
    const sent = [];
    const page = {
      async send(_method, params) {
        sent.push(params.type);
        if (params.type === failedType) throw new Error(`failed ${failedType}`);
      },
    };
    await assert.rejects(
      tapWebViewPoint(page, { x: 24, y: 32 }),
      new RegExp(`failed ${failedType}; touch_cancel=confirmed`),
    );
    const failedIndex = ["touchStart", "touchEnd"].indexOf(failedType);
    assert.deepEqual(sent, [
      ...["touchStart", "touchEnd"].slice(0, failedIndex + 1),
      "touchCancel",
    ]);
  }
});

test("WebView point tap preserves the primary failure when touch cancellation also fails", async () => {
  const sent = [];
  const page = {
    async send(_method, params) {
      sent.push(params.type);
      if (params.type === "touchEnd") throw new Error("touch end response lost");
      if (params.type === "touchCancel") throw new Error("cancel response lost");
    },
  };

  await assert.rejects(
    tapWebViewPoint(page, { x: 24, y: 32 }),
    /touch end response lost; touch_cancel=failed/,
  );
  assert.deepEqual(sent, ["touchStart", "touchEnd", "touchCancel"]);
});

test("WebView point tap contacts a canonical point reacquired immediately before touch", async () => {
  const sent = [];
  const page = {
    async send(_method, params) {
      sent.push({ type: params.type, point: params.touchPoints[0] ?? null });
    },
  };

  await tapWebViewPoint(
    page,
    { x: 24, y: 32 },
    { beforeContact: async () => ({ x: 124, y: 132 }) },
  );

  assert.deepEqual(sent, [
    {
      type: "touchStart",
      point: { x: 124, y: 132, radiusX: 1, radiusY: 1, force: 1, id: 0 },
    },
    { type: "touchEnd", point: null },
  ]);
});

test("WebView point tap rejects invalid initial and replacement points before contact", async () => {
  const sent = [];
  const page = { send: async (_method, params) => sent.push(params.type) };
  await assert.rejects(
    tapWebViewPoint(page, { x: Number.NaN, y: 32 }),
    /invalid initial point/,
  );
  assert.deepEqual(sent, []);

  await assert.rejects(
    tapWebViewPoint(
      page,
      { x: 24, y: 32 },
      { beforeContact: async () => ({ x: Number.POSITIVE_INFINITY, y: 132 }) },
    ),
    /invalid point/,
  );
  assert.deepEqual(sent, []);
});

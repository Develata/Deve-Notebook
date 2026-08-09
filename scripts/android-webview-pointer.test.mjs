import assert from "node:assert/strict";
import test from "node:test";

import { clickWebViewPoint } from "./lib/android-webview-pointer.mjs";

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

import assert from "node:assert/strict";
import test from "node:test";

import {
  armNativeBackDeliveryObservation,
  cancelNativeBackDeliveryObservation,
  observeNativeBackDelivery,
  waitNativeBackDeliveryObservation,
} from "./lib/android-native-back-observation.mjs";

const observationKey = "__DEVE_ANDROID_NATIVE_BACK_OBSERVATION__";

function withGlobalEventTarget(run) {
  const target = new EventTarget();
  const previous = {
    addEventListener: globalThis.addEventListener,
    removeEventListener: globalThis.removeEventListener,
  };
  globalThis.addEventListener = target.addEventListener.bind(target);
  globalThis.removeEventListener = target.removeEventListener.bind(target);
  return Promise.resolve()
    .then(() => run(target))
    .finally(() => {
      globalThis[observationKey]?.controller?.abort();
      delete globalThis[observationKey];
      if (previous.addEventListener === undefined) delete globalThis.addEventListener;
      else globalThis.addEventListener = previous.addEventListener;
      if (previous.removeEventListener === undefined) delete globalThis.removeEventListener;
      else globalThis.removeEventListener = previous.removeEventListener;
    });
}

function nativeBackEvent(detail) {
  const event = new Event("deve-native-back-request");
  Object.defineProperty(event, "detail", { value: detail });
  return event;
}

test("native Back observation projects one synchronous handled acknowledgement", async () => {
  await withGlobalEventTarget(async (target) => {
    const token = 7;
    assert.deepEqual(armNativeBackDeliveryObservation(token), { kind: "armed", token });
    const detail = {
      requestId: "secret-request-id",
      listenerSeen: true,
      outcome: "Handled",
    };
    target.dispatchEvent(nativeBackEvent(detail));
    assert.deepEqual(
      await waitNativeBackDeliveryObservation(token, 100),
      { kind: "delivered", listenerSeen: true, outcome: "Handled" },
    );
    assert.equal(globalThis[observationKey], undefined);
  });
});

test("native Back observation reports bounded missing delivery and retires its listener", async () => {
  await withGlobalEventTarget(async () => {
    const token = 8;
    assert.deepEqual(armNativeBackDeliveryObservation(token), { kind: "armed", token });
    assert.deepEqual(await waitNativeBackDeliveryObservation(token, 5), { kind: "missing" });
    assert.equal(globalThis[observationKey], undefined);
    assert.equal(cancelNativeBackDeliveryObservation(token), false);
  });
});

test("native Back observation refuses to replace an active owner", async () => {
  await withGlobalEventTarget(async () => {
    assert.deepEqual(armNativeBackDeliveryObservation(9), { kind: "armed", token: 9 });
    assert.deepEqual(armNativeBackDeliveryObservation(10), { kind: "busy" });
    assert.equal(cancelNativeBackDeliveryObservation(10), false);
    assert.equal(cancelNativeBackDeliveryObservation(9), true);
  });
});

test("native Back host observation dispatches once and excludes request identity", async () => {
  await withGlobalEventTarget(async (target) => {
    const page = {
      async call(fn, ...args) {
        return fn(...args);
      },
    };
    let dispatches = 0;
    const result = await observeNativeBackDelivery(page, () => {
      dispatches += 1;
      target.dispatchEvent(nativeBackEvent({
        requestId: "must-not-project",
        listenerSeen: true,
        outcome: "Handled",
      }));
    }, 100);
    assert.deepEqual(result, { kind: "delivered", listenerSeen: true, outcome: "Handled" });
    assert.equal(dispatches, 1);
    assert.equal("requestId" in result, false);
  });
});

test("native Back host observation fails with a fixed driver category and cleans up", async () => {
  await withGlobalEventTarget(async () => {
    const page = {
      async call(fn, ...args) {
        return fn(...args);
      },
    };
    await assert.rejects(
      observeNativeBackDelivery(page, () => {
        throw new Error("secret=/private/runner/path");
      }, 100),
      (error) => {
        assert.equal(error.message, "android_platform_back_driver_failed");
        assert.doesNotMatch(error.message, /secret|private|runner|path/);
        return true;
      },
    );
    assert.equal(globalThis[observationKey], undefined);
  });
});

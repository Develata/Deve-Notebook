const NATIVE_BACK_OBSERVATION_TIMEOUT_MS = 1500;

let nextObservationToken = 1;

function projectNativeBackDelivery(value) {
  if (value?.kind === "missing") return { kind: "missing" };
  if (value?.kind !== "delivered"
    || value.listenerSeen !== true
    || !["Handled", "Unhandled"].includes(value.outcome)) return null;
  return {
    kind: "delivered",
    listenerSeen: true,
    outcome: value.outcome,
  };
}

export function armNativeBackDeliveryObservation(token) {
  const key = "__DEVE_ANDROID_NATIVE_BACK_OBSERVATION__";
  const eventName = "deve-native-back-request";
  if (!Number.isSafeInteger(token) || token <= 0) return { kind: "invalid" };
  if (globalThis[key]) return { kind: "busy" };
  const controller = new AbortController();
  const state = {
    token,
    controller,
    result: null,
    resolve: null,
    timeout: null,
  };
  globalThis[key] = state;
  globalThis.addEventListener(eventName, (event) => {
    queueMicrotask(() => {
      if (globalThis[key] !== state || state.result !== null) return;
      const detail = event?.detail;
      state.result = {
        kind: "delivered",
        listenerSeen: detail?.listenerSeen === true,
        outcome: typeof detail?.outcome === "string" ? detail.outcome : null,
      };
      state.resolve?.(state.result);
    });
  }, { signal: controller.signal });
  return { kind: "armed", token };
}

export function waitNativeBackDeliveryObservation(token, timeoutMs) {
  const key = "__DEVE_ANDROID_NATIVE_BACK_OBSERVATION__";
  const state = globalThis[key];
  if (!state || state.token !== token || !Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    return Promise.resolve({ kind: "invalid" });
  }
  return new Promise((resolve) => {
    const finish = (result) => {
      if (globalThis[key] !== state) return;
      if (state.timeout !== null) clearTimeout(state.timeout);
      state.controller.abort();
      delete globalThis[key];
      resolve(result);
    };
    if (state.result !== null) {
      finish(state.result);
      return;
    }
    state.resolve = finish;
    state.timeout = setTimeout(() => finish({ kind: "missing" }), timeoutMs);
  });
}

export function cancelNativeBackDeliveryObservation(token) {
  const key = "__DEVE_ANDROID_NATIVE_BACK_OBSERVATION__";
  const state = globalThis[key];
  if (!state || state.token !== token) return false;
  if (state.timeout !== null) clearTimeout(state.timeout);
  state.controller.abort();
  delete globalThis[key];
  return true;
}

export async function observeNativeBackDelivery(
  page,
  dispatchBack,
  timeoutMs = NATIVE_BACK_OBSERVATION_TIMEOUT_MS,
) {
  const token = nextObservationToken;
  nextObservationToken += 1;
  const armed = await page.call(armNativeBackDeliveryObservation, token).catch(() => null);
  if (armed?.kind !== "armed" || armed.token !== token) {
    throw new Error("android_platform_back_observation_arm_failed");
  }
  try {
    dispatchBack();
  } catch {
    await page.call(cancelNativeBackDeliveryObservation, token).catch(() => {});
    throw new Error("android_platform_back_driver_failed");
  }
  const observed = await page.call(
    waitNativeBackDeliveryObservation,
    token,
    timeoutMs,
  ).catch(() => null);
  const projected = projectNativeBackDelivery(observed);
  if (!projected) {
    await page.call(cancelNativeBackDeliveryObservation, token).catch(() => {});
    throw new Error("android_platform_back_observation_invalid");
  }
  return projected;
}

import { invoke } from "@tauri-apps/api/core";

const unavailable = (error = "native backend bridge unavailable") => ({
  available: false,
  error,
});

const hasTauriInvoke = () =>
  typeof window !== "undefined" &&
  typeof window.__TAURI_INTERNALS__?.invoke === "function";

async function callNative(command, args = {}) {
  if (!hasTauriInvoke()) return unavailable();
  try {
    return {
      available: true,
      value: await invoke(command, args),
    };
  } catch (error) {
    return unavailable(String(error?.message || error));
  }
}

window.__DEVE_NATIVE_BACKEND_CONFIG__ = {
  available: hasTauriInvoke,
  getConfig: async () => callNative("native_backend_get_config"),
  validateRemote: async (remoteUrl) =>
    callNative("native_backend_validate_remote", { remoteUrl }),
  saveRemote: async (remoteUrl) =>
    callNative("native_backend_save_remote", { remoteUrl }),
  switchLocal: async () => callNative("native_backend_switch_local"),
};

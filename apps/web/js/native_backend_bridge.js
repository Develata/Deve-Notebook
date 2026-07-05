import { invoke } from "@tauri-apps/api/core";

// plan_ref:
//   - 15_settings#native-host-local-backend-preference
//   - 11_ui_design/02_desktop#desktop-native-shell-modes
//   - 11_ui_design/03_mobile#mobile-native-shell-modes

const unavailable = (error = "native backend bridge unavailable") => ({
  available: false,
  error,
});

const browserWindow = typeof window === "undefined" ? null : window;

const hasTauriInvoke = () =>
  typeof browserWindow?.__TAURI_INTERNALS__?.invoke === "function";

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

const nativeBackendConfigFacade = {
  available: hasTauriInvoke,
  getConfig: async () => callNative("native_backend_get_config"),
  validateRemote: async (remoteUrl) =>
    callNative("native_backend_validate_remote", { remoteUrl }),
  saveRemote: async (remoteUrl) =>
    callNative("native_backend_save_remote", { remoteUrl }),
  switchLocal: async () => callNative("native_backend_switch_local"),
};

const bridge = browserWindow?.__deveWebBridge;
if (!bridge || typeof bridge.register !== "function") {
  throw new Error(
    "web bridge registry unavailable before registering native backend config",
  );
}

bridge.register("__DEVE_NATIVE_BACKEND_CONFIG__", nativeBackendConfigFacade, {
  runtime: "native_shell_mode_runtime",
  source: "native-backend-bridge",
  authority: "none",
  role: "host-local-backend-preference-facade",
});

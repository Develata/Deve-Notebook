// plan_ref: 11_ui_design/03_mobile#mobile-native-shell-modes
((root, fallback, installId, replace) => {
  const key = "__DEVE_NATIVE_BOOTSTRAP_CURRENT__";
  const identityValid = typeof installId === "string" && /^[0-9a-f]{32}$/.test(installId);
  const parse = (raw) => {
    try {
      return raw ? JSON.parse(raw) : null;
    } catch (_error) {
      return null;
    }
  };
  const localBase = (value, protocol) => {
    try {
      const url = new URL(value);
      return url.protocol === protocol
        && url.hostname === "127.0.0.1"
        && url.port !== ""
        && url.username === ""
        && url.password === ""
        && url.pathname === "/"
        && url.search === ""
        && url.hash === "";
    } catch (_error) {
      return false;
    }
  };
  const valid = (bootstrap) => bootstrap
    && typeof bootstrap === "object"
    && !Array.isArray(bootstrap)
    && Object.keys(bootstrap).sort().join(",")
      === "capabilities,http_base,node_role,platform_lifecycle_authority,session_bound,ws_base"
    && localBase(bootstrap.http_base, "http:")
    && localBase(bootstrap.ws_base, "ws:")
    && bootstrap.node_role === fallback.node_role
    && bootstrap.platform_lifecycle_authority === "native"
    && bootstrap.platform_lifecycle_authority
      === fallback.platform_lifecycle_authority
    && bootstrap.session_bound === true
    && bootstrap.capabilities
    && typeof bootstrap.capabilities === "object"
    && !Array.isArray(bootstrap.capabilities)
    && Object.keys(bootstrap.capabilities).join(",") === "backend_preference_control"
    && bootstrap.capabilities.backend_preference_control
      === fallback.capabilities.backend_preference_control;
  const validEnvelope = (value) => value
    && typeof value === "object"
    && !Array.isArray(value)
    && Object.keys(value).sort().join(",") === "bootstrap,session_install_id"
    && typeof value.session_install_id === "string"
    && valid(value.bootstrap);
  const fail = () => {
    root.__DEVE_NATIVE_BOOTSTRAP = {
      service_state: "session_invalid",
      platform_lifecycle_authority: "native",
      capabilities: fallback.capabilities,
    };
    root.dispatchEvent(new root.Event("deve-native-service-error"));
  };

  let current = fallback;
  let ready = false;
  root.__DEVE_NATIVE_SESSION_INSTALL_ID = installId;
  try {
    const saved = parse(root.sessionStorage.getItem(key));
    if (replace || !validEnvelope(saved) || saved.session_install_id !== installId) {
      root.sessionStorage.setItem(key, JSON.stringify({
        session_install_id: installId,
        bootstrap: fallback,
      }));
    }
    const confirmed = parse(root.sessionStorage.getItem(key));
    if (identityValid && validEnvelope(confirmed) && confirmed.session_install_id === installId) {
      current = confirmed.bootstrap;
      ready = true;
    }
  } catch (_error) {}
  root.__DEVE_NATIVE_SESSION_STORAGE_READY = ready;
  if (ready) root.__DEVE_NATIVE_BOOTSTRAP = current;
  else fail();
})

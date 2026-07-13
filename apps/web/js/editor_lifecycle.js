// CodeMirror mount ownership. Projection-only lifecycle state; no write authority.

function retireActiveView(state) {
  const view = state.activeView;
  state.activeView = null;
  state.activeHost = null;
  if (view) view.destroy();
}

export function activateEditorMount(state, host, createView) {
  if (!host) throw new TypeError("Editor mount host is required");
  if (host.isConnected === false) {
    throw new TypeError("Editor mount host is disconnected");
  }
  if (typeof createView !== "function") {
    throw new TypeError("Editor view factory is required");
  }

  retireActiveView(state);
  const view = createView();
  if (!view || typeof view.destroy !== "function") {
    throw new TypeError("Editor view factory returned an invalid view");
  }
  state.activeHost = host;
  state.activeView = view;
  return view;
}

export function destroyOwnedEditorMount(state, expectedHost) {
  if (!expectedHost || state.activeHost !== expectedHost || !state.activeView) {
    return false;
  }
  retireActiveView(state);
  return true;
}

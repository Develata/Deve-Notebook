// CodeMirror mount ownership. Projection-only lifecycle state; no write authority.

let nextGestureSelectionToken = 0;
let activeGestureSelection = null;

export function captureGestureEditorSelection(state) {
  const view = state.activeView;
  const selection = view?.state?.selection;
  const doc = view?.state?.doc;
  const docLength = doc?.length;
  if (!view || !selection || !Number.isSafeInteger(docLength) || docLength < 0) {
    activeGestureSelection = null;
    return null;
  }

  nextGestureSelectionToken += 1;
  if (!Number.isSafeInteger(nextGestureSelectionToken)) {
    nextGestureSelectionToken = 1;
  }
  const token = nextGestureSelectionToken;
  activeGestureSelection = { token, view, selection, doc, docLength };
  return token;
}

export function settleGestureEditorSelection(state, token, restore) {
  const snapshot = activeGestureSelection;
  if (!snapshot || !Number.isSafeInteger(token) || snapshot.token !== token) {
    return false;
  }
  activeGestureSelection = null;
  if (!restore) return true;

  const view = state.activeView;
  if (
    view !== snapshot.view
    || view?.state?.doc !== snapshot.doc
    || view?.state?.doc?.length !== snapshot.docLength
    || typeof view?.dispatch !== "function"
  ) {
    return false;
  }
  view.dispatch({ selection: snapshot.selection });
  return true;
}

export function retireGestureEditorSelection() {
  activeGestureSelection = null;
}

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

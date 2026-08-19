export function withCreateDom(elements, hit, run) {
  const originals = {
    document: globalThis.document,
    getComputedStyle: globalThis.getComputedStyle,
    window: globalThis.window,
    requestAnimationFrame: globalThis.requestAnimationFrame,
    observation: globalThis.__deveAndroidCreatePointerObservation,
    lane: globalThis.__deveAndroidCreatePointerLane,
  };
  const listeners = new Map();
  globalThis.getComputedStyle = () => ({ display: "block", visibility: "visible" });
  globalThis.window = { innerWidth: 400, innerHeight: 800 };
  globalThis.requestAnimationFrame = (callback) => setImmediate(callback);
  globalThis.document = {
    querySelector: (selector) => selector === "[data-deve-sync-status]" ? ({
      getAttribute(name) {
        if (name === "data-deve-sync-status") return "ready";
        if (name === "data-deve-repo-id") return "repo-1";
        if (name === "data-deve-scope-nonce") return "7";
        return null;
      },
    }) : null,
    querySelectorAll: () => elements,
    elementFromPoint: (...point) => typeof hit === "function" ? hit(...point) : hit,
    addEventListener: (type, listener, options = {}) => {
      if (!listeners.has(type)) listeners.set(type, new Map());
      listeners.get(type).set(listener, options?.once === true);
    },
    removeEventListener: (type, listener) => {
      listeners.get(type)?.delete(listener);
    },
    emitEvent(type, target) {
      const event = {
        type,
        target,
        timeStamp: Date.now(),
        defaultPrevented: false,
        immediatePropagationStopped: false,
        preventDefault() { this.defaultPrevented = true; },
        stopImmediatePropagation() { this.immediatePropagationStopped = true; },
      };
      const typedListeners = listeners.get(type) ?? new Map();
      for (const [listener, once] of [...typedListeners]) {
        if (once) typedListeners.delete(listener);
        listener(event);
        if (event.immediatePropagationStopped) break;
      }
      if (type === "click" && !event.immediatePropagationStopped) target.commitClick();
      return event;
    },
    emitClick(target) { return this.emitEvent("click", target); },
    listenerCount: (type) => listeners.get(type)?.size ?? 0,
    clickListenerCount: () => listeners.get("click")?.size ?? 0,
  };
  return Promise.resolve(run()).finally(() => {
    for (const [name, value] of Object.entries(originals)) {
      const target = name === "observation"
        ? "__deveAndroidCreatePointerObservation"
        : name === "lane" ? "__deveAndroidCreatePointerLane" : name;
      if (value === undefined) delete globalThis[target];
      else globalThis[target] = value;
    }
  });
}

export function createResult(target, { left = 10, onCommit = () => {} } = {}) {
  let currentLeft = left;
  const element = {
    getAttribute: (name) => name === "data-deve-search-result-create-target" ? target : null,
    getBoundingClientRect: () => ({
      left: currentLeft,
      top: 20,
      right: currentLeft + 100,
      bottom: 64,
      width: 100,
      height: 44,
    }),
    contains: () => false,
    closest: () => element,
    commitClick: onCommit,
    emitEvent: (type) => globalThis.document.emitEvent(type, element),
    emitClick: () => globalThis.document.emitClick(element),
    getLeft: () => currentLeft,
    setLeft: (nextLeft) => { currentLeft = nextLeft; },
  };
  return element;
}

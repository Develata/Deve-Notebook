class FakeElement {
  constructor(onClick = () => {}) {
    this.onClick = onClick;
  }

  click() {
    this.onClick();
  }
}

class FakeButton extends FakeElement {
  constructor(onClick, disabled = false) {
    super(onClick);
    this.disabled = disabled;
  }
}

export function installAndroidRemovalDom({
  scopeSamples = [
    { status: "snapshot-loading", repoId: "repo-1", scopeNonce: "3" },
    { status: "ready", repoId: "repo-1", scopeNonce: "3" },
  ],
  mobile = false,
} = {}) {
  const original = {
    document: globalThis.document,
    visible: globalThis.__deveVisibleElement,
    HTMLElement: globalThis.HTMLElement,
    HTMLButtonElement: globalThis.HTMLButtonElement,
  };
  const state = {
    sample: 0,
    actionsOpen: false,
    removalPreview: false,
    noScope: false,
    drawerOpen: false,
    drawerOpenCount: 0,
    drawerCloseCount: 0,
    explorerActive: false,
    switcherOpen: false,
  };
  const status = {
    getAttribute(attribute) {
      const sample = state.noScope
        ? { status: "handshaking-repo", repoId: "", scopeNonce: "4" }
        : scopeSamples[Math.min(state.sample, scopeSamples.length - 1)];
      const value = {
        "data-deve-sync-status": sample.status,
        "data-deve-repo-id": sample.repoId,
        "data-deve-scope-nonce": sample.scopeNonce,
      }[attribute] ?? null;
      if (attribute === "data-deve-scope-nonce" && !state.noScope) state.sample += 1;
      return value;
    },
  };
  const actions = new FakeElement(() => {
    state.actionsOpen = true;
  });
  const item = {
    parentElement: { querySelector: () => actions },
    getAttribute: (attribute) => attribute === "data-deve-repo-switcher-item-name"
      ? "only-repo"
      : null,
  };
  const confirm = new FakeButton(() => {
    state.removalPreview = false;
    state.noScope = true;
    state.switcherOpen = false;
  });
  const dialog = {
    querySelector(selector) {
      if (selector === "#repo-removal-preserved-heading") return new FakeElement();
      if (selector === '[data-deve-repo-removal-confirm="true"]') return confirm;
      return null;
    },
  };
  const trigger = new FakeButton(() => {
    state.switcherOpen = true;
  });
  const create = new FakeButton();
  const remove = new FakeButton(() => {
    state.removalPreview = true;
  });
  const backdrop = new FakeButton(() => {
    state.switcherOpen = false;
  });
  const openDrawer = new FakeButton(() => {
    state.drawerOpen = true;
    state.drawerOpenCount += 1;
  });
  const closeDrawer = new FakeButton(() => {
    state.drawerOpen = false;
    state.drawerCloseCount += 1;
  });
  const explorerTab = new FakeButton(() => {
    state.explorerActive = true;
  });
  explorerTab.getAttribute = (attribute) => attribute === "data-deve-mobile-sidebar-tab-active"
    ? String(state.explorerActive)
    : null;
  const drawer = {
    getAttribute: (attribute) => attribute === "data-deve-mobile-drawer-open"
      ? String(state.drawerOpen)
      : null,
  };

  globalThis.HTMLElement = FakeElement;
  globalThis.HTMLButtonElement = FakeButton;
  globalThis.document = {
    querySelector: (selector) => ({
      "[data-deve-sync-status]": status,
      '[data-deve-mobile-drawer="left"]': mobile ? drawer : null,
      '[data-deve-mobile-sidebar-tab="explorer"]': mobile ? explorerTab : null,
    })[selector] ?? null,
    querySelectorAll: (selector) => selector === "[data-deve-repo-switcher-item]" && !state.noScope
      ? [item]
      : [],
  };
  globalThis.__deveVisibleElement = (selector) => ({
    '[data-deve-layout-mode="mobile"]': mobile ? new FakeElement() : null,
    '[data-deve-mobile-header-action="open_left_drawer"]': mobile ? openDrawer : null,
    '[data-deve-mobile-sidebar-tab="explorer"]': mobile ? explorerTab : null,
    '[data-deve-mobile-drawer="left"] [data-deve-mobile-touch-target="drawer_close_buttons"]':
      mobile && state.drawerOpen ? closeDrawer : null,
    "[data-deve-repo-switcher-trigger]": trigger,
    "[data-deve-repo-switcher-create]": !mobile || state.switcherOpen ? create : null,
    "[data-deve-repo-switcher-remove]": state.actionsOpen ? remove : null,
    '[data-deve-repo-removal-dialog="visible"]': state.removalPreview ? dialog : null,
    '[data-deve-repo-removal-confirm="true"]': state.removalPreview ? confirm : null,
    "[data-deve-repo-switcher-backdrop]": !mobile || state.switcherOpen ? backdrop : null,
  })[selector] ?? null;

  return {
    state,
    restore() {
      for (const [name, value] of Object.entries(original)) {
        const target = name === "visible" ? "__deveVisibleElement" : name;
        if (value === undefined) delete globalThis[target];
        else globalThis[target] = value;
      }
    },
  };
}

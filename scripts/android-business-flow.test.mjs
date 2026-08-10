import assert from "node:assert/strict";
import test from "node:test";

import {
  createFirstAndroidRepoFromBootstrapUnbound,
  exerciseAndroidLastRepoRemoval,
  waitForStableAndroidRepoScope,
} from "./lib/android-business-flow.mjs";

class FakeElement {
  constructor(onClick = () => {}) {
    this.onClick = onClick;
  }

  click() {
    this.onClick();
  }

  dispatchEvent() {
    return true;
  }
}

class FakeInput extends FakeElement {
  constructor(form) {
    super();
    this.form = form;
    this.currentValue = "";
  }

  closest(selector) {
    return selector === "form" ? this.form : null;
  }
}

Object.defineProperty(FakeInput.prototype, "value", {
  get() {
    return this.currentValue;
  },
  set(value) {
    this.currentValue = value;
  },
});

class FakeTextArea extends FakeInput {}

class FakeButton extends FakeElement {
  constructor(onClick, disabled = false) {
    super(onClick);
    this.disabled = disabled;
  }
}

function installFakeDom({
  initialRepoIdRaw = "",
  initialScopeNonceRaw = "0",
  submitDisabled = false,
  becomeReady = true,
} = {}) {
  const original = {
    document: globalThis.document,
    visible: globalThis.__deveVisibleElement,
    HTMLElement: globalThis.HTMLElement,
    HTMLInputElement: globalThis.HTMLInputElement,
    HTMLTextAreaElement: globalThis.HTMLTextAreaElement,
    HTMLButtonElement: globalThis.HTMLButtonElement,
    InputEvent: globalThis.InputEvent,
  };
  const state = {
    created: false,
    submitted: false,
    backdropClosed: false,
    alias: "",
  };
  const submit = new FakeButton(() => {
    state.submitted = true;
    if (becomeReady) {
      state.created = true;
      state.alias = input.value;
    }
  }, submitDisabled);
  const form = {
    querySelector: (selector) => selector === 'button[type="submit"]' ? submit : null,
  };
  const input = new FakeInput(form);
  const create = new FakeButton();
  const trigger = new FakeButton();
  const backdrop = new FakeButton(() => {
    state.backdropClosed = true;
  });
  const status = {
    getAttribute(attribute) {
      if (attribute === "data-deve-sync-status") {
        return state.created ? "ready" : "handshaking-repo";
      }
      if (attribute === "data-deve-repo-id") {
        return state.created ? "repo-1" : initialRepoIdRaw;
      }
      if (attribute === "data-deve-scope-nonce") {
        return state.created ? "1" : initialScopeNonceRaw;
      }
      return null;
    },
  };
  const repoItem = {
    getAttribute: (attribute) => attribute === "data-deve-repo-switcher-item-name"
      ? state.alias
      : null,
  };

  globalThis.HTMLElement = FakeElement;
  globalThis.HTMLInputElement = FakeInput;
  globalThis.HTMLTextAreaElement = FakeTextArea;
  globalThis.HTMLButtonElement = FakeButton;
  globalThis.InputEvent = class extends Event {};
  globalThis.document = {
    querySelector: (selector) => selector === "[data-deve-sync-status]" ? status : null,
    querySelectorAll: (selector) => selector === "[data-deve-repo-switcher-item]"
      && state.created ? [repoItem] : [],
  };
  globalThis.__deveVisibleElement = (selector) => ({
    "[data-deve-repo-switcher-trigger]": trigger,
    "[data-deve-repo-switcher-create]": create,
    "[data-deve-repo-switcher-create-input]": input,
    "[data-deve-repo-switcher-backdrop]": backdrop,
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

const page = { call: (fn, ...args) => fn(...args) };
const waitUntil = async (label, predicate) => {
  const value = await predicate();
  if (value) return value;
  throw new Error(`timeout waiting for ${label}`);
};
const repoCreateOptions = { waitUntil, stableQuietMs: 0 };

async function withFakeDom(options, action) {
  const fixture = installFakeDom(options);
  try {
    return await action(fixture.state);
  } finally {
    fixture.restore();
  }
}

test("first Android repo transitions from explicit BootstrapUnbound(0) to writer ready", async () => {
  await withFakeDom({}, async (state) => {
    const result = await createFirstAndroidRepoFromBootstrapUnbound(
      page,
      "first-repo",
      repoCreateOptions,
    );
    assert.equal(result.initial.repoIdRaw, "");
    assert.equal(result.initial.scopeNonceRaw, "0");
    assert.equal(result.created.repoId, "repo-1");
    assert.equal(result.created.scopeNonce, 1);
    assert.equal(state.submitted, true);
    assert.equal(state.backdropClosed, true);
  });
});

for (const [label, options] of [
  ["missing repo id", { initialRepoIdRaw: null }],
  ["missing nonce", { initialScopeNonceRaw: null }],
  ["empty nonce", { initialScopeNonceRaw: "" }],
  ["non-numeric nonce", { initialScopeNonceRaw: "zero" }],
]) {
  test(`BootstrapUnbound fails closed for ${label}`, async () => {
    await withFakeDom(options, async () => {
      await assert.rejects(
        createFirstAndroidRepoFromBootstrapUnbound(page, "first-repo", repoCreateOptions),
        /timeout waiting for initial zero-repo BootstrapUnbound/,
      );
    });
  });
}

test("first Android repo fails closed when the visible submit is disabled", async () => {
  await withFakeDom({ submitDisabled: true }, async () => {
    await assert.rejects(
      createFirstAndroidRepoFromBootstrapUnbound(page, "first-repo", repoCreateOptions),
      /first Create must use the visible repo switcher form/,
    );
  });
});

test("first Android repo fails closed when backend writer readiness never arrives", async () => {
  await withFakeDom({ becomeReady: false }, async () => {
    await assert.rejects(
      createFirstAndroidRepoFromBootstrapUnbound(page, "first-repo", repoCreateOptions),
      /timeout waiting for first Android repo writer readiness/,
    );
  });
});

test("Android repo writer admission resets its quiet window after a scope change", async () => {
  const observations = [
    { status: "ready", repoIdRaw: "repo-1", scopeNonceRaw: "2" },
    { status: "ready", repoIdRaw: "repo-1", scopeNonceRaw: "3" },
    { status: "ready", repoIdRaw: "repo-1", scopeNonceRaw: "3" },
    { status: "ready", repoIdRaw: "repo-1", scopeNonceRaw: "3" },
  ];
  let now = 0;
  const observationPage = {
    async call() {
      return observations.shift() ?? {
        status: "ready",
        repoIdRaw: "repo-1",
        scopeNonceRaw: "3",
      };
    },
  };
  const pollingWait = async (_label, predicate) => {
    for (let attempt = 0; attempt < 6; attempt += 1) {
      const value = await predicate();
      if (value) return value;
      now += 500;
    }
    throw new Error("stable scope not observed");
  };

  const stable = await waitForStableAndroidRepoScope(observationPage, pollingWait, {
    expectedRepoId: "repo-1",
    minimumScopeNonce: 2,
    quietMs: 1000,
    now: () => now,
  });

  assert.equal(stable.scopeNonce, 3);
  assert.equal(now, 1500);
});

function installRemovalDom({
  scopeSamples = [
    { status: "snapshot-loading", repoId: "repo-1", scopeNonce: "3" },
    { status: "ready", repoId: "repo-1", scopeNonce: "3" },
  ],
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
  });
  const dialog = {
    querySelector(selector) {
      if (selector === "#repo-removal-preserved-heading") return new FakeElement();
      if (selector === '[data-deve-repo-removal-confirm="true"]') return confirm;
      return null;
    },
  };
  const trigger = new FakeButton();
  const create = new FakeButton();
  const remove = new FakeButton(() => {
    state.removalPreview = true;
  });
  const backdrop = new FakeButton();

  globalThis.HTMLElement = FakeElement;
  globalThis.HTMLButtonElement = FakeButton;
  globalThis.document = {
    querySelector: (selector) => selector === "[data-deve-sync-status]" ? status : null,
    querySelectorAll: (selector) => selector === "[data-deve-repo-switcher-item]" && !state.noScope
      ? [item]
      : [],
  };
  globalThis.__deveVisibleElement = (selector) => ({
    '[data-deve-layout-mode="mobile"]': null,
    "[data-deve-repo-switcher-trigger]": trigger,
    "[data-deve-repo-switcher-create]": create,
    "[data-deve-repo-switcher-remove]": state.actionsOpen ? remove : null,
    '[data-deve-repo-removal-dialog="visible"]': state.removalPreview ? dialog : null,
    '[data-deve-repo-removal-confirm="true"]': state.removalPreview ? confirm : null,
    "[data-deve-repo-switcher-backdrop]": backdrop,
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

test("Android last-repo removal waits through snapshot loading for the expected writer scope", async () => {
  const fixture = installRemovalDom();
  let observedAt = -500;
  const pollingWait = async (label, predicate) => {
    for (let attempt = 0; attempt < 8; attempt += 1) {
      const value = await predicate();
      if (value) return value;
    }
    throw new Error(`timeout waiting for ${label}`);
  };
  try {
    const result = await exerciseAndroidLastRepoRemoval(page, {
      waitUntil: pollingWait,
      expectedRepoId: "repo-1",
      minimumScopeNonce: 3,
      stableNow: () => (observedAt += 500),
    });
    assert.equal(result.removedRepoId, "repo-1");
    assert.equal(result.scopeNonceBeforeRemoval, 3);
    assert.equal(result.scopeNonceAfterRemoval, 4);
    assert.equal(observedAt, 1000);
    assert.equal(fixture.state.sample >= 2, true);
  } finally {
    fixture.restore();
  }
});

test("Android last-repo removal keeps every intent closed while snapshot loading persists", async () => {
  const fixture = installRemovalDom({
    scopeSamples: [{ status: "snapshot-loading", repoId: "repo-1", scopeNonce: "3" }],
  });
  const boundedWait = async (label, predicate) => {
    for (let attempt = 0; attempt < 3; attempt += 1) await predicate();
    throw new Error(`timeout waiting for ${label}`);
  };
  try {
    await assert.rejects(
      exerciseAndroidLastRepoRemoval(page, {
        waitUntil: boundedWait,
        expectedRepoId: "repo-1",
        minimumScopeNonce: 3,
        stableQuietMs: 0,
      }),
      /timeout waiting for stable Android repo writer scope/,
    );
    assert.equal(fixture.state.actionsOpen, false);
    assert.equal(fixture.state.removalPreview, false);
  } finally {
    fixture.restore();
  }
});

test("Android last-repo removal revalidates scope before preview intent", async () => {
  const fixture = installRemovalDom({
    scopeSamples: [
      { status: "ready", repoId: "repo-1", scopeNonce: "3" },
      { status: "snapshot-loading", repoId: "repo-1", scopeNonce: "3" },
    ],
  });
  try {
    await assert.rejects(
      exerciseAndroidLastRepoRemoval(page, {
        waitUntil,
        expectedRepoId: "repo-1",
        minimumScopeNonce: 3,
        stableQuietMs: 0,
      }),
      /Android repo removal preview intent rejected: status-not-ready/,
    );
    assert.equal(fixture.state.actionsOpen, true);
    assert.equal(fixture.state.removalPreview, false);
  } finally {
    fixture.restore();
  }
});

test("Android last-repo removal revalidates exact scope before execute intent", async () => {
  const fixture = installRemovalDom({
    scopeSamples: [
      { status: "ready", repoId: "repo-1", scopeNonce: "3" },
      { status: "ready", repoId: "repo-1", scopeNonce: "3" },
      { status: "ready", repoId: "repo-1", scopeNonce: "4" },
    ],
  });
  try {
    await assert.rejects(
      exerciseAndroidLastRepoRemoval(page, {
        waitUntil,
        expectedRepoId: "repo-1",
        minimumScopeNonce: 3,
        stableQuietMs: 0,
      }),
      /Android repo removal execute intent rejected: scope-nonce-mismatch/,
    );
    assert.equal(fixture.state.removalPreview, true);
    assert.equal(fixture.state.noScope, false);
  } finally {
    fixture.restore();
  }
});

test("Android last-repo removal rejects a different backend repo scope", async () => {
  const fixture = installRemovalDom({
    scopeSamples: [{ status: "ready", repoId: "repo-2", scopeNonce: "9" }],
  });
  const boundedWait = async (label, predicate) => {
    for (let attempt = 0; attempt < 3; attempt += 1) {
      if (await predicate()) throw new Error("unexpected destructive admission");
    }
    throw new Error(`timeout waiting for ${label}`);
  };
  try {
    await assert.rejects(
      exerciseAndroidLastRepoRemoval(page, {
        waitUntil: boundedWait,
        expectedRepoId: "repo-1",
        minimumScopeNonce: 3,
        stableQuietMs: 0,
      }),
      /timeout waiting for stable Android repo writer scope/,
    );
    assert.equal(fixture.state.actionsOpen, false);
    assert.equal(fixture.state.removalPreview, false);
  } finally {
    fixture.restore();
  }
});

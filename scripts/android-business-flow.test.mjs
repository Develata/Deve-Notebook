import assert from "node:assert/strict";
import test from "node:test";

import { createFirstAndroidRepoFromBootstrapUnbound } from "./lib/android-business-flow.mjs";

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
      { waitUntil },
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
        createFirstAndroidRepoFromBootstrapUnbound(page, "first-repo", { waitUntil }),
        /timeout waiting for initial zero-repo BootstrapUnbound/,
      );
    });
  });
}

test("first Android repo fails closed when the visible submit is disabled", async () => {
  await withFakeDom({ submitDisabled: true }, async () => {
    await assert.rejects(
      createFirstAndroidRepoFromBootstrapUnbound(page, "first-repo", { waitUntil }),
      /first Create must use the visible repo switcher form/,
    );
  });
});

test("first Android repo fails closed when backend writer readiness never arrives", async () => {
  await withFakeDom({ becomeReady: false }, async () => {
    await assert.rejects(
      createFirstAndroidRepoFromBootstrapUnbound(page, "first-repo", { waitUntil }),
      /timeout waiting for first Android repo writer readiness/,
    );
  });
});

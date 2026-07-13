import assert from "node:assert/strict";
import test from "node:test";

import { probeWebCryptoEd25519 } from "./lib/webcrypto-capability.mjs";

test("WebCrypto capability probe requires non-extractable Ed25519 signing keys", async () => {
  const calls = [];
  const result = await probeWebCryptoEd25519({
    async generateKey(algorithm, extractable, usages) {
      calls.push({ algorithm, extractable, usages });
      return { privateKey: { extractable: false } };
    },
  });

  assert.equal(result.writable, true);
  assert.equal(result.blocker, null);
  assert.deepEqual(calls, [{
    algorithm: { name: "Ed25519" },
    extractable: false,
    usages: ["sign", "verify"],
  }]);
});

test("missing WebCrypto is a stable fail-closed blocker", async () => {
  const result = await probeWebCryptoEd25519(null);

  assert.equal(result.writable, false);
  assert.equal(result.blocker, "webcrypto_unavailable");
});

test("unsupported Ed25519 is a stable fail-closed blocker", async () => {
  const result = await probeWebCryptoEd25519({
    async generateKey() {
      const error = new Error("unsupported");
      error.name = "NotSupportedError";
      throw error;
    },
  });

  assert.equal(result.writable, false);
  assert.equal(result.blocker, "ed25519_unavailable");
  assert.equal(result.errorName, "NotSupportedError");
});

test("transient Ed25519 probe errors do not claim the algorithm is unsupported", async () => {
  const result = await probeWebCryptoEd25519({
    async generateKey() {
      const error = new Error("operation failed");
      error.name = "OperationError";
      throw error;
    },
  });

  assert.equal(result.writable, false);
  assert.equal(result.blocker, "capability_probe_failed");
  assert.equal(result.errorName, "OperationError");
});

test("an extractable private key is rejected even when Ed25519 generation succeeds", async () => {
  const result = await probeWebCryptoEd25519({
    async generateKey() {
      return { privateKey: { extractable: true } };
    },
  });

  assert.equal(result.writable, false);
  assert.equal(result.blocker, "capability_probe_failed");
  assert.equal(result.errorName, "InvalidKeyResult");
});

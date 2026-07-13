export async function probeWebCryptoEd25519(subtle = globalThis.crypto?.subtle) {
  const userAgent = globalThis.navigator?.userAgent ?? "unknown";
  if (!subtle) {
    return {
      writable: false,
      blocker: "webcrypto_unavailable",
      userAgent,
    };
  }

  try {
    const pair = await subtle.generateKey(
      { name: "Ed25519" },
      false,
      ["sign", "verify"],
    );
    if (!pair?.privateKey || pair.privateKey.extractable !== false) {
      return {
        writable: false,
        blocker: "capability_probe_failed",
        errorName: "InvalidKeyResult",
        userAgent,
      };
    }
    return {
      writable: true,
      blocker: null,
      userAgent,
    };
  } catch (error) {
    const errorName = String(error?.name || "Error");
    return {
      writable: false,
      blocker: errorName === "NotSupportedError"
        ? "ed25519_unavailable"
        : "capability_probe_failed",
      errorName,
      userAgent,
    };
  }
}

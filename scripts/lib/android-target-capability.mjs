const PACKAGE_NAME = /\b((?:com\.[a-z0-9_.]+(?:webview|chrome)[a-z0-9_.]*|org\.chromium\.[a-z0-9_.]+))\b/i;
const VERSION_NAME = /\b(\d+(?:\.\d+){1,4})\b/;

function firstMatch(value, pattern) {
  return String(value ?? "").match(pattern)?.[1] ?? null;
}

export function parseAndroidSdkLevel(raw) {
  const value = Number.parseInt(firstMatch(raw, /\b(\d{1,3})\b/) ?? "", 10);
  return Number.isInteger(value) && value > 0 ? value : null;
}

export function parseCurrentWebViewProvider(raw) {
  const source = String(raw ?? "");
  const currentLine = source.split(/\r?\n/).find((line) => /Current WebView package/i.test(line));
  if (!currentLine || /Current WebView package\s+is\s+null\b/i.test(currentLine)) {
    return { packageName: null, versionName: null };
  }
  const tuple = currentLine.match(/Current WebView package[^:]*:\s*\(\s*([a-z0-9_.]+)\s*,\s*(\d+(?:\.\d+){1,4})\s*\)/i);
  if (tuple) {
    return { packageName: tuple[1], versionName: tuple[2] };
  }

  const packageName = firstMatch(currentLine, PACKAGE_NAME);
  if (!packageName) return { packageName: null, versionName: null };
  const packageOffset = currentLine.toLowerCase().indexOf(packageName.toLowerCase());
  const versionName = firstMatch(currentLine.slice(packageOffset + packageName.length), VERSION_NAME);
  return { packageName, versionName };
}

export function buildAndroidTargetFacts({
  sdkRaw,
  webViewRaw,
  avdName,
  buildFingerprint,
  model,
}) {
  const sdkLevel = parseAndroidSdkLevel(sdkRaw);
  const provider = parseCurrentWebViewProvider(webViewRaw);
  const providerMajor = provider.versionName
    ? Number.parseInt(provider.versionName.split(".", 1)[0], 10)
    : null;
  const supportBaseline = Number.isInteger(sdkLevel)
    && sdkLevel >= 29
    && Number.isInteger(providerMajor)
    && providerMajor >= 137;
  return {
    sdkLevel,
    webViewProviderPackage: provider.packageName,
    webViewProviderVersion: provider.versionName,
    webViewProviderMajor: providerMajor,
    avdName: String(avdName ?? "").trim() || null,
    buildFingerprint: String(buildFingerprint ?? "").trim() || null,
    model: String(model ?? "").trim() || null,
    supportBaseline,
  };
}

export function evaluateWritableProbeExpectation(expectWritable, capability) {
  if (expectWritable && !capability?.writable) {
    throw new Error(`writable Android target failed Ed25519 probe: ${capability?.blocker ?? "unknown"}`);
  }
  if (!expectWritable && capability?.writable) {
    throw new Error("negative Android target unexpectedly passed the Ed25519 writer probe");
  }
  if (!expectWritable && !["ed25519_unavailable", "webcrypto_unavailable"].includes(capability?.blocker)) {
    throw new Error(
      `negative Android evidence requires a stable unsupported blocker; observed ${capability?.blocker ?? "unknown"}`,
    );
  }
  return expectWritable ? "writable" : "readonly-negative";
}

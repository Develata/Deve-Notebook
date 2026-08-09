export const ANDROID_REMOTE_BROWSER_MODE_MARKER =
  /^deve_mobile native shell mode=RemoteBrowser embedded_backend=absent$/;
export const ANDROID_EMBEDDED_BACKEND_STARTED_MARKER =
  /^deve_mobile native embedded backend supervisor=started$/;

export function requireAndroidRemoteBrowserModeEvidence(matches) {
  if (!Array.isArray(matches) || matches.length !== 2
    || matches.some((matched) => typeof matched !== "boolean")) {
    throw new Error("Android native mode evidence must contain exactly two booleans");
  }
  const [remoteModeAdmitted, embeddedBackendStarted] = matches;
  if (!remoteModeAdmitted) {
    throw new Error(
      "preference-driven RemoteBrowser must publish its exact native mode admission",
    );
  }
  if (embeddedBackendStarted) {
    throw new Error(
      "preference-driven RemoteBrowser must not start LocalBackend before native intent",
    );
  }
}

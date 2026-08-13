import {
  readEditorMountObservation,
  sameEditorLoadSession,
} from "./mobile-webview-interaction.mjs";

export async function proveAndroidImeBackPriority(
  page,
  {
    waitUntil,
    platformBack,
    activateKeyboard,
    observeEditor = readEditorMountObservation,
    minHeightDelta = 80,
  },
) {
  const before = await waitUntil("Android IME-visible editor session", async () => {
    const observation = await observeEditor(page);
    return observation.activeEditor
      && observation.bridgeReady
      && observation.activeHostMatchesVisible
      && observation.innerHeight - observation.visualViewportHeight >= minHeightDelta
      ? observation
      : null;
  }, 10000);

  await platformBack();
  const hidden = await waitUntil("Android IME-only back dismissal", async () => {
    const observation = await observeEditor(page);
    return observation.activeEditor
      && observation.bridgeReady
      && observation.activeHostMatchesVisible
      && observation.innerHeight - observation.visualViewportHeight < minHeightDelta
      ? observation
      : null;
  }, 10000);
  if (!sameEditorLoadSession(before, hidden)) {
    throw new Error("IME Back replaced the editor host or OpenDoc request");
  }

  await activateKeyboard(hidden.point, page);
  const reopened = await waitUntil("Android IME reopened on the same editor", async () => {
    const observation = await observeEditor(page);
    return observation.activeEditor
      && observation.bridgeReady
      && observation.activeHostMatchesVisible
      && observation.innerHeight - observation.visualViewportHeight >= minHeightDelta
      ? observation
      : null;
  }, 10000);
  if (!sameEditorLoadSession(hidden, reopened)) {
    throw new Error("IME reopen replaced the editor host or OpenDoc request");
  }
  return { before, hidden, reopened };
}

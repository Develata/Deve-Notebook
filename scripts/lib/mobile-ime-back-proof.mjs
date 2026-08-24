import {
  readEditorMountObservation,
  sameEditorLoadSession,
  sameEditorSelectionIdentity,
  sameNativePresentationGeneration,
} from "./mobile-webview-interaction.mjs";
import { keyboardPresentationIsVisible } from "./mobile-keyboard-presentation.mjs";

function summarizeImeObservation(reference, observation) {
  return {
    activeEditor: observation?.activeEditor === true,
    bridgeReady: observation?.bridgeReady === true,
    activeHostMatchesVisible: observation?.activeHostMatchesVisible === true,
    keyboardPresentation: observation?.keyboardPresentation ?? null,
    keyboardOffset: observation?.keyboardOffset ?? null,
    presentationGeneration: observation?.presentationGeneration ?? null,
    presentationEpoch: observation?.presentationEpoch ?? null,
    selectionIdentity: observation?.selectionIdentity ?? null,
    loadSessionMatches: sameEditorLoadSession(reference, observation),
    selectionMatches: sameEditorSelectionIdentity(reference, observation),
    presentationGenerationMatches: sameNativePresentationGeneration(reference, observation),
  };
}

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
      && Number.isSafeInteger(observation.presentationGeneration)
      && Number.isSafeInteger(observation.presentationEpoch)
      && keyboardPresentationIsVisible(observation, minHeightDelta)
      ? observation
      : null;
  }, 10000);

  await platformBack();
  let lastHiddenObservation = null;
  let hidden;
  try {
    hidden = await waitUntil("Android IME-only back dismissal", async () => {
      lastHiddenObservation = await observeEditor(page);
      return lastHiddenObservation.activeEditor
        && lastHiddenObservation.bridgeReady
        && lastHiddenObservation.activeHostMatchesVisible
        && !keyboardPresentationIsVisible(lastHiddenObservation, minHeightDelta)
        && (lastHiddenObservation.keyboardOffset ?? 0) < minHeightDelta
        && sameEditorLoadSession(before, lastHiddenObservation)
        && sameEditorSelectionIdentity(before, lastHiddenObservation)
        && sameNativePresentationGeneration(before, lastHiddenObservation)
        && Number.isSafeInteger(lastHiddenObservation.presentationEpoch)
        && lastHiddenObservation.presentationEpoch >= before.presentationEpoch
        ? lastHiddenObservation
        : null;
    }, 10000);
  } catch (error) {
    throw new Error(
      `${error.message}; android_ime_back_observation=${JSON.stringify(
        summarizeImeObservation(before, lastHiddenObservation),
      )}`,
      { cause: error },
    );
  }

  await activateKeyboard(hidden.point, page);
  let lastReopenedObservation = null;
  let reopened;
  try {
    reopened = await waitUntil("Android IME reopened on the same editor", async () => {
      lastReopenedObservation = await observeEditor(page);
      return lastReopenedObservation.activeEditor
        && lastReopenedObservation.bridgeReady
        && lastReopenedObservation.activeHostMatchesVisible
        && keyboardPresentationIsVisible(lastReopenedObservation, minHeightDelta)
        && sameEditorLoadSession(hidden, lastReopenedObservation)
        && sameNativePresentationGeneration(before, lastReopenedObservation)
        && Number.isSafeInteger(lastReopenedObservation.presentationEpoch)
        && lastReopenedObservation.presentationEpoch >= hidden.presentationEpoch
        ? lastReopenedObservation
        : null;
    }, 10000);
  } catch (error) {
    throw new Error(
      `${error.message}; android_ime_reopen_observation=${JSON.stringify(
        summarizeImeObservation(hidden, lastReopenedObservation),
      )}`,
      { cause: error },
    );
  }
  return { before, hidden, reopened };
}

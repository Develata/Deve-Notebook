import {
  focusEditor,
  readEditorMountObservation,
  sameEditorLoadSession,
  sameEditorSelectionIdentity,
  sameNativePresentationGeneration,
} from "./mobile-webview-interaction.mjs";

export function keyboardPresentationIsVisible(observation, minHeightDelta = 80) {
  if (observation?.keyboardPresentation === "visual-viewport") {
    return Number.isSafeInteger(observation?.keyboardOffset)
      && observation.keyboardOffset >= 0
      && observation.nativeKeyboardOverlay === 0;
  }
  return observation?.keyboardPresentation === "native-insets"
    && Number.isSafeInteger(observation?.keyboardOffset)
    && observation.keyboardOffset >= minHeightDelta
    && observation.nativeKeyboardOverlay === observation.keyboardOffset;
}

export async function proveSameBreakpointKeyboardPresentation(
  page,
  {
    waitUntil,
    activateKeyboard,
    maxViewportWidth = 768,
    minHeightDelta = 80,
    viewportResizeEpsilon = 1,
  },
) {
  const baseline = await waitUntil("Android keyboard hidden baseline", async () => {
    const observation = await readEditorMountObservation(page);
    return observation.keyboardPresentation === "hidden"
      && observation.keyboardOffset === 0
      && observation.nativeKeyboardOverlay === 0
      && Number.isSafeInteger(observation.presentationGeneration)
      && Number.isSafeInteger(observation.presentationEpoch)
      && observation.innerHeight - observation.visualViewportHeight <= viewportResizeEpsilon
      ? observation
      : null;
  }, 10000);
  const point = await focusEditor(page, { requireFocused: false });
  await activateKeyboard(point, page);
  const focused = await readEditorMountObservation(page);
  let lastObservation = null;
  let presented;
  try {
    presented = await waitUntil("same-breakpoint Android keyboard presentation", async () => {
      lastObservation = await readEditorMountObservation(page);
      const visualViewportAccepted =
        lastObservation.keyboardPresentation === "visual-viewport"
        && lastObservation.nativeKeyboardOverlay === 0
        && lastObservation.toolbarBottomGap !== null
        && (
          lastObservation.visualViewportHeight < baseline.visualViewportHeight - minHeightDelta
          || lastObservation.keyboardOffset >= minHeightDelta
        );
      const viewportUnchanged =
        lastObservation.visualViewportHeight
          >= baseline.visualViewportHeight - viewportResizeEpsilon;
      const nativeInsetsAccepted =
        lastObservation.keyboardPresentation === "native-insets"
        && viewportUnchanged
        && lastObservation.keyboardOffset >= minHeightDelta
        && lastObservation.nativeKeyboardOverlay === lastObservation.keyboardOffset
        && lastObservation.toolbarBottomGap >= lastObservation.keyboardOffset - 2;
      return lastObservation.activeEditor
        && lastObservation.bridgeReady
        && lastObservation.activeHostMatchesVisible
        && sameEditorLoadSession(focused, lastObservation)
        && sameEditorSelectionIdentity(focused, lastObservation)
        && sameNativePresentationGeneration(baseline, lastObservation)
        && Number.isSafeInteger(lastObservation.presentationEpoch)
        && lastObservation.presentationEpoch >= baseline.presentationEpoch
        && (visualViewportAccepted || nativeInsetsAccepted)
        ? {
            ...lastObservation,
            keyboardMode: visualViewportAccepted ? "visual-viewport" : "native-insets",
          }
        : null;
    }, 10000);
  } catch (error) {
    throw new Error(
      `${error.message}; android_keyboard_presentation_observation=${JSON.stringify({
        baseline: {
          hostId: baseline.hostId,
          openRequestId: baseline.openRequestId,
          innerHeight: baseline.innerHeight,
          visualViewportHeight: baseline.visualViewportHeight,
          keyboardPresentation: baseline.keyboardPresentation,
          keyboardOffset: baseline.keyboardOffset,
          nativeKeyboardOverlay: baseline.nativeKeyboardOverlay,
          docId: baseline.docId,
          repoId: baseline.repoId,
          scopeNonce: baseline.scopeNonce,
          presentationGeneration: baseline.presentationGeneration,
          presentationEpoch: baseline.presentationEpoch,
        },
        focused: {
          hostId: focused.hostId,
          openRequestId: focused.openRequestId,
          docId: focused.docId,
          repoId: focused.repoId,
          scopeNonce: focused.scopeNonce,
          presentationGeneration: focused.presentationGeneration,
          presentationEpoch: focused.presentationEpoch,
          selectionIdentity: focused.selectionIdentity,
        },
        last: lastObservation && {
          hostId: lastObservation.hostId,
          openRequestId: lastObservation.openRequestId,
          activeEditor: lastObservation.activeEditor,
          bridgeReady: lastObservation.bridgeReady,
          activeHostMatchesVisible: lastObservation.activeHostMatchesVisible,
          innerHeight: lastObservation.innerHeight,
          visualViewportHeight: lastObservation.visualViewportHeight,
          keyboardPresentation: lastObservation.keyboardPresentation,
          keyboardOffset: lastObservation.keyboardOffset,
          nativeKeyboardOverlay: lastObservation.nativeKeyboardOverlay,
          toolbarBottomGap: lastObservation.toolbarBottomGap,
          docId: lastObservation.docId,
          repoId: lastObservation.repoId,
          scopeNonce: lastObservation.scopeNonce,
          presentationGeneration: lastObservation.presentationGeneration,
          presentationEpoch: lastObservation.presentationEpoch,
          selectionIdentity: lastObservation.selectionIdentity,
        },
      })}`,
      { cause: error },
    );
  }
  if (!sameEditorLoadSession(focused, presented)
    || !sameEditorSelectionIdentity(focused, presented)
    || !sameNativePresentationGeneration(baseline, presented)) {
    throw new Error(
      "same-breakpoint keyboard presentation replaced editor/doc/repo/scope/selection/generation",
    );
  }
  if (
    !Number.isSafeInteger(baseline.presentationEpoch)
    || !Number.isSafeInteger(presented.presentationEpoch)
    || presented.presentationEpoch < baseline.presentationEpoch
  ) {
    throw new Error("same-breakpoint keyboard presentation epoch did not remain current");
  }
  if (baseline.viewportWidth > maxViewportWidth || presented.viewportWidth > maxViewportWidth) {
    throw new Error("keyboard presentation crossed the expected mobile breakpoint");
  }
  return { baseline, focused, presented };
}

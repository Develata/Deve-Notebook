import { Prec, StateEffect, StateField } from "@codemirror/state";
import { keymap } from "@codemirror/view";

const TASK_PREFIX = "- [ ] ";
const setToolbarTaskMarker = StateEffect.define();

export const toolbarTaskMarkerField = StateField.define({
  create() {
    return null;
  },
  update(marker, transaction) {
    let markerEffectSeen = false;
    for (const effect of transaction.effects) {
      if (effect.is(setToolbarTaskMarker)) {
        marker = effect.value;
        markerEffectSeen = true;
      }
    }
    if (markerEffectSeen) return marker;
    if (marker === null) return null;

    if (transaction.docChanged) {
      const taskLine = transaction.startState.doc.lineAt(marker);
      let taskLineChanged = false;
      transaction.changes.iterChangedRanges((fromA, toA) => {
        if (fromA <= taskLine.to && toA >= taskLine.from) {
          taskLineChanged = true;
        }
      });
      if (taskLineChanged) return null;
      marker = transaction.changes.mapPos(marker, 1);
    }

    const selection = transaction.newSelection.main;
    if (!selection.empty || selection.head !== marker) return null;
    const line = transaction.newDoc.lineAt(marker);
    return marker === line.to && /^\s*[-+*]\s+\[[ xX]\]\s*$/.test(line.text)
      ? marker
      : null;
  },
});

function isEmptyTaskAtMarker(state, marker) {
  const line = state.doc.lineAt(marker);
  if (marker !== line.to || !state.selection.main.empty) return false;
  return /^\s*[-+*]\s+\[[ xX]\]\s*$/.test(line.text);
}

export function continueToolbarTaskItem(view) {
  const marker = view.state.field(toolbarTaskMarkerField, false);
  const selection = view.state.selection.main;
  if (
    marker === null
    || selection.head !== marker
    || !isEmptyTaskAtMarker(view.state, marker)
  ) {
    return false;
  }

  const line = view.state.doc.lineAt(marker);
  const indentation = line.text.match(/^\s*/)?.[0] || "";
  const insert = `\n${indentation}${TASK_PREFIX}`;
  view.dispatch({
    changes: { from: marker, insert },
    selection: { anchor: marker + insert.length },
    effects: setToolbarTaskMarker.of(null),
    userEvent: "input.type",
  });
  return true;
}

export function insertToolbarTaskItem(view) {
  if (!view || view.state?.readOnly) return false;
  const selection = view.state.selection.main;
  const marker = selection.from + TASK_PREFIX.length;
  view.dispatch({
    changes: { from: selection.from, to: selection.to, insert: TASK_PREFIX },
    selection: { anchor: marker },
    effects: setToolbarTaskMarker.of(marker),
    userEvent: "input.type",
  });
  view.focus();
  return true;
}

export const mobileTaskItemExtension = [
  toolbarTaskMarkerField,
  Prec.highest(keymap.of([{ key: "Enter", run: continueToolbarTaskItem }])),
];

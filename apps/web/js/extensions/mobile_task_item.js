import { Prec, StateEffect, StateField } from "@codemirror/state";
import { keymap } from "@codemirror/view";

const TASK_PREFIX = "- [ ] ";
const CONTINUE_READY = "continue-ready";
const EXIT_READY = "exit-ready";
const setToolbarTaskMarker = StateEffect.define({
  map(marker, changes) {
    return marker === null
      ? null
      : { ...marker, pos: changes.mapPos(marker.pos, 1) };
  },
});

export const toolbarTaskMarkerField = StateField.define({
  create() {
    return null;
  },
  update(marker, transaction) {
    const previousMarker = marker;
    let markerEffectSeen = false;
    for (const effect of transaction.effects) {
      if (effect.is(setToolbarTaskMarker)) {
        marker = effect.value;
        markerEffectSeen = true;
      }
    }
    if (marker === null) return null;

    if (!markerEffectSeen && transaction.docChanged) {
      const taskLine = transaction.startState.doc.lineAt(previousMarker.pos);
      let taskLineChanged = false;
      transaction.changes.iterChangedRanges((fromA, toA) => {
        if (fromA <= taskLine.to && toA >= taskLine.from) {
          taskLineChanged = true;
        }
      });
      if (taskLineChanged) return null;
      marker = {
        ...marker,
        pos: transaction.changes.mapPos(marker.pos, 1),
      };
    }

    const selection = transaction.newSelection.main;
    if (
      transaction.newSelection.ranges.length !== 1
      || !selection.empty
      || selection.head !== marker.pos
    ) return null;
    const line = transaction.newDoc.lineAt(marker.pos);
    return marker.pos === line.to && /^\s*[-+*]\s+\[[ xX]\]\s*$/.test(line.text)
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
    || view.state.selection.ranges.length !== 1
    || selection.head !== marker.pos
    || !isEmptyTaskAtMarker(view.state, marker.pos)
  ) {
    return false;
  }

  const line = view.state.doc.lineAt(marker.pos);
  if (marker.phase === EXIT_READY) {
    view.dispatch({
      changes: { from: line.from, to: line.to, insert: "" },
      selection: { anchor: line.from },
      effects: setToolbarTaskMarker.of(null),
      userEvent: "input.type",
    });
    return true;
  }
  if (marker.phase !== CONTINUE_READY) return false;

  const indentation = line.text.match(/^\s*/)?.[0] || "";
  const insert = `\n${indentation}${TASK_PREFIX}`;
  view.dispatch({
    changes: { from: marker.pos, insert },
    selection: { anchor: marker.pos + insert.length },
    effects: setToolbarTaskMarker.of({
      pos: marker.pos + insert.length,
      phase: EXIT_READY,
    }),
    userEvent: "input.type",
  });
  return true;
}

export function insertToolbarTaskItem(view) {
  if (!view || view.state?.readOnly) return false;
  const selection = view.state.selection.main;
  if (view.state.selection.ranges.length !== 1 || !selection.empty) return false;
  const marker = selection.from + TASK_PREFIX.length;
  view.dispatch({
    changes: { from: selection.from, to: selection.to, insert: TASK_PREFIX },
    selection: { anchor: marker },
    effects: setToolbarTaskMarker.of({ pos: marker, phase: CONTINUE_READY }),
    userEvent: "input.type",
  });
  view.focus();
  return true;
}

export const mobileTaskItemExtension = [
  toolbarTaskMarkerField,
  Prec.highest(keymap.of([{ key: "Enter", run: continueToolbarTaskItem }])),
];

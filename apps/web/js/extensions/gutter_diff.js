import { StateEffect, StateField } from "@codemirror/state";
import { GutterMarker, gutter } from "@codemirror/view";
import { ctx } from "../editor_state.js";

export const updateGutterDiffEffect = StateEffect.define();

const gutterRangesField = StateField.define({
  create() {
    return [];
  },
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(updateGutterDiffEffect)) {
        return Array.isArray(effect.value) ? effect.value : [];
      }
    }
    return value;
  },
});

class DiffMarker extends GutterMarker {
  constructor(kind) {
    super();
    this.kind = kind;
    this.color = gutterDiffColorForKind(kind);
  }
  toDOM() {
    const el = document.createElement("div");
    el.dataset.deveGutterDiffKind = this.kind;
    el.style.width = "4px";
    el.style.height = "100%";
    el.style.backgroundColor = this.color;
    return el;
  }
}

export const gutterDiffColorTokens = Object.freeze({
  added: "var(--color-added)",
  deleted: "var(--color-deleted)",
  modified: "var(--color-modified)",
});

export function gutterDiffColorForKind(kind) {
  return gutterDiffColorTokens[kind] ?? null;
}

const addedMarker = new DiffMarker("added");
const deletedMarker = new DiffMarker("deleted");
const modifiedMarker = new DiffMarker("modified");

function markerForKind(kind) {
  if (kind === "added") return addedMarker;
  if (kind === "deleted") return deletedMarker;
  if (kind === "modified") return modifiedMarker;
  return null;
}

export const gutterDiffExtension = [
  gutterRangesField,
  gutter({
    lineMarker(view, line) {
      const ranges = view.state.field(gutterRangesField);
      const hit = ranges.find((r) => line.number >= r.start_line && line.number <= r.end_line);
      return hit ? markerForKind(hit.kind) : null;
    },
  }),
];

export function updateGutterDiff(rangesJson) {
  if (!ctx.activeView) return;
  let parsed = [];
  try {
    const value = JSON.parse(rangesJson);
    parsed = Array.isArray(value) ? value : [];
  } catch (_e) {
    parsed = [];
  }
  ctx.activeView.dispatch({ effects: updateGutterDiffEffect.of(parsed) });
}

window.updateGutterDiff = updateGutterDiff;

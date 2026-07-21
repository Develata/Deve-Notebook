import { Decoration } from "@codemirror/view";
import { renderRangeIndexField } from "./render_range_index.js";
import { renderFailureEffect, renderThemeEffect } from "./render_effects.js";

export function createRenderFieldState(state, kind, buildDecorations) {
    const index = state.field(renderRangeIndexField);
    const failed = new Set();
    return {
        failed,
        decorations: buildAllDecorations(state, index, kind, failed, buildDecorations),
    };
}

export function updateRenderFieldState(current, transaction, kind, buildDecorations, options = {}) {
    const index = transaction.state.field(renderRangeIndexField);
    if (transaction.docChanged) {
        const failed = new Set();
        return {
            failed,
            decorations: buildAllDecorations(
                transaction.state,
                index,
                kind,
                failed,
                buildDecorations,
            ),
        };
    }

    let failed = current.failed;
    const affected = new Map();
    let refreshAll = false;

    for (const effect of transaction.effects) {
        if (effect.is(renderFailureEffect) && effect.value?.kind === kind) {
            const range = index.matchingFailure(effect.value);
            if (range && !failed.has(range.key)) {
                failed = new Set(failed);
                failed.add(range.key);
                affected.set(range.key, range);
            }
        }
        if (effect.is(renderThemeEffect) && options.refreshOnTheme) {
            failed = new Set();
            refreshAll = true;
        }
    }

    if (refreshAll) {
        return {
            failed,
            decorations: buildAllDecorations(
                transaction.state,
                index,
                kind,
                failed,
                buildDecorations,
            ),
        };
    }

    if (transaction.selection) {
        const oldIndex = transaction.startState.field(renderRangeIndexField);
        for (const range of oldIndex.query(kind, transaction.startState.selection.ranges)) {
            affected.set(range.key, range);
        }
        for (const range of index.query(kind, transaction.state.selection.ranges)) {
            affected.set(range.key, range);
        }
    }

    if (affected.size === 0) return current;

    const revealedKeys = revealedRangeKeys(index, kind, transaction.state.selection.ranges);
    const affectedKeys = new Set();
    const add = [];
    let filterFrom = transaction.state.doc.length;
    let filterTo = 0;
    for (const range of affected.values()) {
        const anchor = companionAnchor(transaction.state, range);
        affectedKeys.add(range.key);
        filterFrom = Math.min(filterFrom, range.from, anchor);
        filterTo = Math.max(filterTo, range.to, anchor);
        add.push(...desiredDecorations(
            transaction.state,
            range,
            failed,
            buildDecorations,
            revealedKeys,
        ));
    }
    const decorations = current.decorations.update({
        filterFrom,
        filterTo,
        filter: (_from, _to, value) => !affectedKeys.has(value.spec?.deveRenderKey),
        add,
        sort: true,
    });
    return { failed, decorations };
}

export function isRangeRevealed(state, index, range) {
    return index.query(range.kind, state.selection.ranges)
        .some((candidate) => candidate.key === range.key);
}

export function isCompanionRange(state, range) {
    const main = state.selection.main;
    return main.empty
        && range.type !== "INLINE"
        && main.head >= range.from
        && main.head < range.to;
}

export function companionAnchor(state, range) {
    const closingPosition = Math.max(range.from, range.to - 1);
    return state.doc.lineAt(closingPosition).to;
}

export function taggedReplace(range, widget, block) {
    return Decoration.replace({
        widget,
        block,
        deveRenderKey: range.key,
        deveRenderMode: "replace",
    }).range(range.from, range.to);
}

export function taggedCompanion(state, range, widget) {
    const anchor = companionAnchor(state, range);
    return Decoration.widget({
        widget,
        block: true,
        side: 1,
        deveRenderKey: range.key,
        deveRenderMode: "companion",
    }).range(anchor);
}

function buildAllDecorations(state, index, kind, failed, buildDecorations) {
    const decorations = [];
    const revealedKeys = revealedRangeKeys(index, kind, state.selection.ranges);
    for (const range of index.ranges(kind)) {
        decorations.push(...desiredDecorations(
            state,
            range,
            failed,
            buildDecorations,
            revealedKeys,
        ));
    }
    decorations.sort((a, b) => a.from - b.from || a.startSide - b.startSide);
    return Decoration.set(decorations, true);
}

function desiredDecorations(state, range, failed, buildDecorations, revealedKeys) {
    const companion = isCompanionRange(state, range);
    if (failed.has(range.key) && !companion) return [];
    return buildDecorations({
        state,
        range,
        revealed: revealedKeys.has(range.key),
        companion,
    });
}

function revealedRangeKeys(index, kind, selections) {
    return new Set(index.query(kind, selections).map((range) => range.key));
}

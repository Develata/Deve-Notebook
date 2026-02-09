use leptos::prelude::*;
use std::collections::HashSet;

#[derive(Clone)]
pub struct FoldState {
    pub folding_enabled: ReadSignal<bool>,
    pub context_lines: ReadSignal<usize>,
    pub expanded_folds: ReadSignal<HashSet<usize>>,
    pub toggle_folding: Callback<()>,
    pub on_expand_fold: Callback<usize>,
    pub set_context_lines: Callback<usize>,
    pub clear_expanded: Callback<()>,
}

pub fn create_fold_state() -> FoldState {
    let (folding_enabled, set_folding_enabled) = signal(true);
    let (context_lines, set_context_lines_sig) = signal(5usize);
    let (expanded_folds, set_expanded_folds) = signal(HashSet::<usize>::new());

    let toggle_folding = Callback::new(move |_| {
        set_folding_enabled.update(|v| *v = !*v);
        set_expanded_folds.set(HashSet::new());
    });
    let on_expand_fold = Callback::new(move |id: usize| {
        set_expanded_folds.update(|set| {
            set.insert(id);
        });
    });
    let clear_expanded = Callback::new(move |_| {
        set_expanded_folds.set(HashSet::new());
    });
    let set_context_lines = Callback::new(move |next: usize| {
        set_context_lines_sig.set(next.max(1));
        set_expanded_folds.set(HashSet::new());
    });

    FoldState {
        folding_enabled,
        context_lines,
        expanded_folds,
        toggle_folding,
        on_expand_fold,
        set_context_lines,
        clear_expanded,
    }
}

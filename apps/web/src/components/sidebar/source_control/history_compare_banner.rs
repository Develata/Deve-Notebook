use crate::components::sidebar::source_control::history_compare_logic::short_commit_id;
use crate::i18n::Locale;
use deve_core::source_control::CommitInfo;
use leptos::prelude::*;

#[component]
pub fn HistoryCompareBanner(
    locale: RwSignal<Locale>,
    compare_base_commit_id: RwSignal<Option<String>>,
    commit_history: ReadSignal<Vec<CommitInfo>>,
    selected_commit: RwSignal<Option<String>>,
    clear_compare_base: Callback<()>,
    use_selected_as_base: Callback<()>,
) -> impl IntoView {
    let selected_target = Signal::derive(move || {
        let selected_id = selected_commit.get()?;
        commit_history
            .get()
            .into_iter()
            .find(|commit| commit.id == selected_id)
    });
    let base_commit = Signal::derive(move || {
        let base_id = compare_base_commit_id.get()?;
        commit_history
            .get()
            .into_iter()
            .find(|commit| commit.id == base_id)
    });

    view! {
        <Show when=move || compare_base_commit_id.get().is_some() || selected_target.get().is_some()>
            <div class="mx-4 mt-2 mb-1 rounded-md border border-active bg-hover px-3 py-2 text-[12px] text-secondary flex items-center justify-between gap-3">
                <div class="min-w-0">
                    {move || {
                        if let Some(base) = base_commit.get() {
                            let base_label = short_commit_id(&base.id);
                            if let Some(target) = selected_target.get() {
                                let target_label = short_commit_id(&target.id);
                                view! {
                                    <span>
                                        {match locale.get() {
                                            Locale::En => format!("Comparing {base_label} -> {target_label}."),
                                            Locale::Zh => format!("正在比较 {base_label} -> {target_label}。"),
                                        }}
                                    </span>
                                }
                                .into_any()
                            } else {
                                view! {
                                    <span>
                                        {match locale.get() {
                                            Locale::En => format!(
                                                "Base {base_label} selected. Click another commit to compare."
                                            ),
                                            Locale::Zh => format!(
                                                "已选择基准提交 {base_label}。点击另一条提交即可比较。"
                                            ),
                                        }}
                                    </span>
                                }
                                .into_any()
                            }
                        } else if let Some(target) = selected_target.get() {
                            let target_label = short_commit_id(&target.id);
                            view! {
                                <span>
                                    {match locale.get() {
                                        Locale::En => format!(
                                            "Selected {target_label}. Use it as the comparison base?"
                                        ),
                                        Locale::Zh => format!(
                                            "已选择提交 {target_label}。要把它设为比较基准吗？"
                                        ),
                                    }}
                                </span>
                            }
                            .into_any()
                        } else {
                            view! {}.into_any()
                        }
                    }}
                </div>
                {move || {
                    if compare_base_commit_id.get().is_some() {
                        view! {
                            <button
                                class="shrink-0 text-[11px] font-medium text-accent hover:underline"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    clear_compare_base.run(());
                                }
                            >
                                {match locale.get() {
                                    Locale::En => "Clear",
                                    Locale::Zh => "清除",
                                }}
                            </button>
                        }
                        .into_any()
                    } else if selected_target.get().is_some() {
                        view! {
                            <button
                                class="shrink-0 rounded px-2 py-1 text-[11px] font-medium bg-accent/15 text-accent hover:bg-accent/20"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    use_selected_as_base.run(());
                                }
                            >
                                {match locale.get() {
                                    Locale::En => "Use as Base",
                                    Locale::Zh => "设为基准",
                                }}
                            </button>
                        }
                        .into_any()
                    } else {
                        view! {}.into_any()
                    }
                }}
            </div>
        </Show>
    }
}

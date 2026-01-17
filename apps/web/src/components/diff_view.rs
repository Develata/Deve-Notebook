// apps\web\src\components
//! # Diff 视图组件 (Diff View)
//!
//! 并排显示文档的新旧版本差异。
//!
//! **功能**:
//! - 左侧: 已提交版本 (旧)
//! - 右侧: 当前版本 (新)
//! - 高亮显示差异行

use leptos::prelude::*;

/// Diff 视图组件
#[component]
pub fn DiffView<F>(
    /// 文件路径
    path: String,
    /// 已提交版本内容
    old_content: String,
    /// 当前版本内容
    new_content: String,
    /// 关闭回调
    on_close: F,
) -> impl IntoView 
where
    F: Fn() + 'static + Clone,
{
    let old_lines: Vec<_> = old_content.lines().map(String::from).collect();
    let new_lines: Vec<_> = new_content.lines().map(String::from).collect();
    
    // 简单对比: 按行号匹配
    let max_lines = old_lines.len().max(new_lines.len());
    
    // 提取文件名和目录
    let full_path = path.clone();
    // 处理 Windows 和 Unix 路径分隔符
    let normalized_path = full_path.replace('\\', "/"); 
    let path_parts: Vec<&str> = normalized_path.split('/').collect();
    let filename = path_parts.last().unwrap_or(&"?").to_string();
    let directory = if path_parts.len() > 1 {
        path_parts[..path_parts.len()-1].join("/")
    } else {
        String::new()
    };

    view! {
        // bg-white for Light, bg-[#1e1e1e] for Dark
        <div class="diff-view h-full flex flex-col bg-white dark:bg-[#1e1e1e] text-gray-800 dark:text-[#cccccc] font-sans">
            // 标题栏 - Simplified
            <div class="diff-header flex justify-between items-center border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-[#2d2d2d] px-4 py-2 text-xs select-none">
                <div class="flex items-center gap-2">
                    // File Icon
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 text-blue-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>
                    
                    <span class="font-bold text-gray-700 dark:text-[#cccccc]">{filename}</span>
                    <span class="text-gray-400 dark:text-gray-500 ml-1">{directory}</span>
                    
                    <span class="ml-2 px-1.5 py-0.5 rounded text-[10px] bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400">
                        "Working Tree"
                    </span>
                </div>
                
                // 关闭按钮
                <button 
                    class="p-1 hover:bg-gray-200 dark:hover:bg-[#ffffff1a] rounded text-gray-500 dark:text-gray-400 transition-colors"
                    title="Close Diff View (Esc)"
                    on:click=move |_| on_close()
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                </button>
            </div>
            
            // 内容区域
            <div class="diff-content flex-1 flex overflow-auto font-mono text-sm leading-6">
                // 左侧 (已提交)
                <div class="flex-1 overflow-auto bg-gray-50/30 dark:bg-[#1e1e1e]">
                    {(0..max_lines).map(|i| {
                        let old_line = old_lines.get(i).cloned().unwrap_or_default();
                        let new_line = new_lines.get(i).cloned().unwrap_or_default();
                        let is_diff = old_line != new_line;
                        
                        view! {
                            <div 
                                class="px-4 py-0.5 whitespace-pre" 
                                class:bg-red-100=is_diff
                                class:dark:bg-red-900=is_diff
                                class:dark:bg-opacity-30=is_diff
                            >
                                <span class="text-gray-400 dark:text-gray-500 mr-4 select-none">{format!("{:4}", i + 1)}</span>
                                {old_line}
                            </div>
                        }
                    }).collect_view()}
                </div>
                
                // 右侧 (当前)
                <div class="flex-1 overflow-auto border-l border-gray-200 dark:border-gray-700 bg-white dark:bg-[#1e1e1e]">
                    {(0..max_lines).map(|i| {
                        let old_line = old_lines.get(i).cloned().unwrap_or_default();
                        let new_line = new_lines.get(i).cloned().unwrap_or_default();
                        let is_diff = old_line != new_line;
                        
                        view! {
                            <div 
                                class="px-4 py-0.5 whitespace-pre"
                                class:bg-green-100=is_diff
                                class:dark:bg-green-900=is_diff
                                class:dark:bg-opacity-30=is_diff
                            >
                                <span class="text-gray-400 dark:text-gray-500 mr-4 select-none">{format!("{:4}", i + 1)}</span>
                                {new_line}
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}

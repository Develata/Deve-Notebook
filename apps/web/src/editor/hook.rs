//! # Editor Hook (编辑器钩子)
//!
//! **架构作用**:
//! 封装编辑器的状态管理逻辑 (`use_editor`)。
//! 包含文档加载、WebSocket 消息处理协调、CodeMirror 初始化和更新循环。

use leptos::prelude::*;
use leptos::html::Div;
use wasm_bindgen::prelude::*;
use crate::api::WsService;
use crate::hooks::use_core::CoreState;
use deve_core::protocol::ClientMessage;
use deve_core::models::DocId;
use super::ffi::{setupCodeMirror, set_read_only};
use super::EditorStats;
use super::sync;
use super::playback;

pub struct EditorState {
    pub content: ReadSignal<String>,
    pub is_playback: ReadSignal<bool>,
    pub playback_version: ReadSignal<u64>,
    pub local_version: ReadSignal<u64>, 
    pub on_playback_change: Box<dyn Fn(u64) + Send + Sync>, 
}

pub fn use_editor(
    doc_id: DocId,
    editor_ref: NodeRef<Div>,
    on_stats: Option<Callback<EditorStats>>,
) -> EditorState {
    let ws = use_context::<WsService>().expect("WsService should be provided");
    let core = expect_context::<CoreState>();
    
    // 文档的本地状态，用于计算 diff
    let (content, set_content) = signal("".to_string()); // Start empty
    let (local_version, set_local_version) = signal(0u64);
    
    // 回放状态
    let (history, set_history) = signal(Vec::<(u64, deve_core::models::Op)>::new());
    // 我们使用 Core 的回放版本信号
    // let (playback_version, set_playback_version) = signal(0u64); <-- 已替换
    let playback_version = core.playback_version;
    let set_playback_version = core.set_playback_version;
    
    let (is_playback, set_is_playback) = signal(false);
    
    // 生成会话 client_id (使用随机数)
    let client_id = (js_sys::Math::random() * 1_000_000.0) as u64;
    
    // 初始请求: 打开文档
    // 我们在挂载时发送 OpenDoc，以及当 doc_id 改变时。
    // 注意: Effect 在 props 改变时运行。
    let ws_clone = ws.clone();
    let set_doc_ver = core.set_doc_version;
    Effect::new(move |_| {
         // 文档改变时重置状态
         set_content.set("Loading...".to_string());
         set_local_version.set(0);
         set_history.set(Vec::new());
         
         // 重置本文档的 Core 状态
         set_doc_ver.set(0);
         set_playback_version.set(0);
         
         ws_clone.send(ClientMessage::OpenDoc { doc_id });
    });

    // 将本地版本同步到 Core 文档版本
    Effect::new(move |_| {
         let ver = local_version.get();
         set_doc_ver.set(ver);
    });

    // 处理传入消息的 Effect (委托给 sync 模块)
    let ws_clone_2 = ws.clone();
    Effect::new(move |_| {
         if let Some(msg) = ws_clone_2.msg.get() {
             sync::handle_server_message(
                 msg,
                 doc_id,
                 client_id,
                 &ws_clone_2,
                 set_content,
                 local_version,
                 set_local_version,
                 set_history,
                 is_playback,
                 set_playback_version,
                 on_stats
             );
         }
    });


    Effect::new(move |_| {
        if let Some(element) = editor_ref.get() {
            let raw_element: &web_sys::HtmlElement = &element;
            let ws_for_update = ws.clone();
            
            let on_update = Closure::wrap(Box::new(move |new_text: String| {
                // 如果处于回放模式，忽略更改 (只读)
                if is_playback.get_untracked() {
                    return;
                }

                let old_text = content.get_untracked();
                if new_text == old_text {
                    return;
                }

                // 计算统计信息
                if let Some(cb) = on_stats {
                     let lines = new_text.lines().count();
                     let words = new_text.split_whitespace().count();
                     cb.run(EditorStats { chars: new_text.len(), words, lines });
                }
                
                // 计算 Diff
                let ops = deve_core::state::compute_diff(&old_text, &new_text);
                
                // 发送 Ops
                if !ops.is_empty() {
                    for op in ops {
                        ws_for_update.send(ClientMessage::Edit { 
                            doc_id, 
                            op: op.clone(),
                            client_id 
                        });
                    }
                }
                
                // 更新本地状态
                set_content.set(new_text);
                
            }) as Box<dyn FnMut(String)>);

            setupCodeMirror(raw_element, &on_update);
            on_update.forget(); 
        }
    });

    // 回放逻辑 (监听 Core 回放版本)
    Effect::new(move |_| {
         let ver = playback_version.get();
         let local = local_version.get_untracked();
         
         // 调用逻辑
         playback::handle_playback_change(
            ver,
            doc_id,
            local,
            history, 
            set_is_playback
        );
        
        // 判断只读状态: 回放中 OR 旁观者模式
        let is_pb = ver < local;
        let spectator = core.is_spectator.get_untracked();
        let should_readonly = is_pb || spectator;
        set_read_only(should_readonly);
    });

    let on_playback_change = Box::new(move |ver: u64| {
        set_playback_version.set(ver);
    });

    EditorState {
        content: content,
        is_playback: is_playback,
        playback_version: playback_version,
        local_version: local_version,
        on_playback_change,
    }
}

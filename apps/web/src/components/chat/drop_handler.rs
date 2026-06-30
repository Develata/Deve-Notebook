// apps/web/src/components/chat/drop_handler.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 10_rendering#document-authority-bridge
//!
use crate::hooks::use_core::sync_banner_notice::show_sync_banner;
use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason, cannot_action};
use crate::i18n::Locale;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

type EventClosure = Rc<RefCell<Option<Closure<dyn FnMut(web_sys::Event)>>>>;
const MAX_CHAT_ATTACHMENT_BYTES: f64 = 1024.0 * 1024.0;

fn attach_file_error(locale: Locale, reason: WriteGateReason) -> String {
    cannot_action(locale, WriteGateAction::AttachFile, reason)
}

pub fn on_drag_over(set_is_drag_over: WriteSignal<bool>) -> impl Fn(web_sys::DragEvent) {
    move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_is_drag_over.set(true);
    }
}

pub fn on_drag_leave(set_is_drag_over: WriteSignal<bool>) -> impl Fn(web_sys::DragEvent) {
    move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_is_drag_over.set(false);
    }
}

pub fn on_drop(
    set_input: WriteSignal<String>,
    set_is_drag_over: WriteSignal<bool>,
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
) -> impl Fn(web_sys::DragEvent) {
    move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_is_drag_over.set(false);

        if let Some(data_transfer) = ev.data_transfer()
            && let Some(files) = data_transfer.files()
        {
            for i in 0..files.length() {
                if let Some(file) = files.item(i) {
                    let name = file.name();
                    if file.size() > MAX_CHAT_ATTACHMENT_BYTES {
                        let message = attach_file_error(
                            locale.get_untracked(),
                            WriteGateReason::FileTooLarge,
                        );
                        leptos::logging::warn!("{}: {}", message, name);
                        show_sync_banner(set_sync_banner, message);
                        continue;
                    }

                    let Ok(reader) = web_sys::FileReader::new() else {
                        let message = attach_file_error(
                            locale.get_untracked(),
                            WriteGateReason::FileReaderUnavailable,
                        );
                        leptos::logging::warn!("{}: {}", message, name);
                        show_sync_banner(set_sync_banner, message);
                        continue;
                    };
                    let reader_c = reader.clone();
                    let name_c = name.clone();
                    let set_input = set_input;

                    // 使用 Rc<RefCell> 自清理模式，避免 .forget() 导致的内存泄漏
                    let onload_slot: EventClosure = Rc::new(RefCell::new(None));
                    let onload_slot_c = onload_slot.clone();

                    let onload = Closure::wrap(Box::new(move |_e: web_sys::Event| {
                        if let Ok(content) = reader_c
                            .result()
                            .and_then(|r| r.as_string().ok_or(wasm_bindgen::JsValue::NULL))
                        {
                            set_input.update(|curr| {
                                let block = format!("\n```{}\n{}\n```\n", name_c, content);
                                curr.push_str(&block);
                            });
                        }
                        // 自清理: 释放闭包引用，允许 GC 回收
                        let _ = onload_slot_c.borrow_mut().take();
                    }) as Box<dyn FnMut(_)>);

                    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                    *onload_slot.borrow_mut() = Some(onload);
                    if reader.read_as_text(&file).is_err() {
                        reader.set_onload(None);
                        let _ = onload_slot.borrow_mut().take();
                        let message = attach_file_error(
                            locale.get_untracked(),
                            WriteGateReason::FileReadFailed,
                        );
                        leptos::logging::warn!("{}: {}", message, name);
                        show_sync_banner(set_sync_banner, message);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::attach_file_error;
    use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason};
    use crate::i18n::{Locale, t};

    #[test]
    fn attach_file_errors_are_localized_banner_copy() {
        assert_eq!(
            attach_file_error(Locale::Zh, WriteGateReason::FileTooLarge),
            t::write_gate::cannot_action(
                Locale::Zh,
                WriteGateAction::AttachFile,
                WriteGateReason::FileTooLarge,
            )
        );
        assert_eq!(
            attach_file_error(Locale::En, WriteGateReason::FileReaderUnavailable),
            t::write_gate::cannot_action(
                Locale::En,
                WriteGateAction::AttachFile,
                WriteGateReason::FileReaderUnavailable,
            )
        );
        assert_eq!(
            attach_file_error(Locale::Zh, WriteGateReason::FileReadFailed),
            t::write_gate::cannot_action(
                Locale::Zh,
                WriteGateAction::AttachFile,
                WriteGateReason::FileReadFailed,
            )
        );
    }
}

use super::SocketMessage;
use js_sys::{ArrayBuffer, JsString, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};

pub(super) fn decode_socket_message(data: JsValue) -> Option<SocketMessage> {
    if let Ok(buffer) = data.clone().dyn_into::<ArrayBuffer>() {
        let bytes = Uint8Array::new(&buffer).to_vec();
        return Some(SocketMessage::Bytes(bytes));
    }

    if let Ok(text) = data.dyn_into::<JsString>() {
        return Some(SocketMessage::Text(String::from(&text)));
    }

    None
}

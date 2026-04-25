use self::socket_message::decode_socket_message;
pub use self::socket_types::{SocketCloseInfo, SocketEvent, SocketMessage};
use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{BinaryType, CloseEvent, Event, MessageEvent, WebSocket};

mod socket_message;
mod socket_types;

pub struct BrowserSocket {
    ws: WebSocket,
    _onopen: Closure<dyn FnMut(Event)>,
    _onmessage: Closure<dyn FnMut(MessageEvent)>,
    _onerror: Closure<dyn FnMut(Event)>,
    _onclose: Closure<dyn FnMut(CloseEvent)>,
}

impl BrowserSocket {
    pub fn connect(url: &str) -> Result<(Self, UnboundedReceiver<SocketEvent>), JsValue> {
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);

        let (tx, rx) = unbounded();

        let onopen_tx = tx.clone();
        let onopen = Closure::wrap(Box::new(move |_event: Event| {
            let _ = onopen_tx.unbounded_send(SocketEvent::Opened);
        }) as Box<dyn FnMut(Event)>);
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));

        let onmessage_tx = tx.clone();
        let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Some(message) = decode_socket_message(event.data()) {
                let _ = onmessage_tx.unbounded_send(SocketEvent::Message(message));
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

        let onerror_tx = tx.clone();
        let onerror = Closure::wrap(Box::new(move |_event: Event| {
            let _ = onerror_tx.unbounded_send(SocketEvent::Error);
        }) as Box<dyn FnMut(Event)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        let onclose = Closure::wrap(Box::new(move |event: CloseEvent| {
            let _ = tx.unbounded_send(SocketEvent::Closed(SocketCloseInfo {
                code: event.code(),
                reason: event.reason(),
                was_clean: event.was_clean(),
            }));
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));

        Ok((
            Self {
                ws,
                _onopen: onopen,
                _onmessage: onmessage,
                _onerror: onerror,
                _onclose: onclose,
            },
            rx,
        ))
    }

    pub fn send_binary(&self, bytes: &[u8]) -> Result<(), JsValue> {
        self.ws.send_with_u8_array(bytes)
    }

    pub fn ready_state(&self) -> u16 {
        self.ws.ready_state()
    }

    pub fn is_open(&self) -> bool {
        self.ready_state() == WebSocket::OPEN
    }

    pub fn is_closed(&self) -> bool {
        matches!(self.ready_state(), WebSocket::CLOSING | WebSocket::CLOSED)
    }
}

impl Drop for BrowserSocket {
    fn drop(&mut self) {
        self.ws.set_onopen(None);
        self.ws.set_onmessage(None);
        self.ws.set_onerror(None);
        self.ws.set_onclose(None);
        let _ = self.ws.close();
    }
}

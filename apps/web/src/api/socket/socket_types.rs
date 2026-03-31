pub enum SocketEvent {
    Opened,
    Message(SocketMessage),
    Error,
    Closed(SocketCloseInfo),
}

pub enum SocketMessage {
    Bytes(Vec<u8>),
    Text(String),
}

pub struct SocketCloseInfo {
    pub code: u16,
    pub reason: String,
    pub was_clean: bool,
}

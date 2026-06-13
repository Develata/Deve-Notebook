use futures::{Sink, Stream};
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::Sleep;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

pub(crate) struct MockSocket {
    incoming: VecDeque<Result<Message, WsError>>,
    pub(crate) sent: Vec<Message>,
}

pub(crate) enum DelayedFrame {
    Ready(Message),
    After {
        sleep: Pin<Box<Sleep>>,
        message: Option<Message>,
    },
}

pub(crate) struct DelayedSocket {
    incoming: VecDeque<DelayedFrame>,
    pub(crate) sent: Vec<Message>,
}

impl MockSocket {
    pub(crate) fn new(incoming: Vec<Message>) -> Self {
        Self {
            incoming: incoming.into_iter().map(Ok).collect(),
            sent: Vec::new(),
        }
    }
}

impl DelayedSocket {
    pub(crate) fn new(incoming: Vec<DelayedFrame>) -> Self {
        Self {
            incoming: incoming.into(),
            sent: Vec::new(),
        }
    }
}

impl Stream for MockSocket {
    type Item = Result<Message, WsError>;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.incoming.pop_front())
    }
}

impl Sink<Message> for MockSocket {
    type Error = WsError;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        self.sent.push(item);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Stream for DelayedSocket {
    type Item = Result<Message, WsError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(frame) = self.incoming.front_mut() else {
            return Poll::Ready(None);
        };
        match frame {
            DelayedFrame::Ready(_) => match self.incoming.pop_front() {
                Some(DelayedFrame::Ready(message)) => Poll::Ready(Some(Ok(message))),
                _ => unreachable!("front frame is ready"),
            },
            DelayedFrame::After { sleep, message } => {
                if sleep.as_mut().poll(cx).is_pending() {
                    return Poll::Pending;
                }
                let message = message.take().expect("delayed message");
                self.incoming.pop_front();
                Poll::Ready(Some(Ok(message)))
            }
        }
    }
}

impl Sink<Message> for DelayedSocket {
    type Error = WsError;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        self.sent.push(item);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

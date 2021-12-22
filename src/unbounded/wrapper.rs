use crate::error::SendError;
use crate::UnboundedSender;
use futures_sink::Sink;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct UnboundedSenderWrapper<T> {
    inner: Option<UnboundedSender<T>>,
}

impl<T> Sink<T> for UnboundedSenderWrapper<T> {
    type Error = SendError;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .as_ref()
            .and_then(|sender| (!sender.is_closed()).then(|| Poll::Ready(Ok(()))))
            .unwrap_or(Poll::Ready(Err(SendError::Disconnected)))
    }

    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.inner
            .as_mut()
            .map(|sender| sender.send(item).map_err(|err| err.into_send_error()))
            .unwrap_or(Err(SendError::Disconnected))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner = None;
        Poll::Ready(Ok(()))
    }
}

impl<T> UnboundedSenderWrapper<T> {
    pub fn new(sender: UnboundedSender<T>) -> Self {
        Self {
            inner: Some(sender),
        }
    }
}

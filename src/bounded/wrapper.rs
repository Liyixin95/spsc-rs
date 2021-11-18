use crate::bounded::ring::Indexer;
use crate::bounded::Sender;
use crate::error::SendError;
use futures_sink::Sink;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct SenderWrapper<T, I: Indexer> {
    inner: Option<Sender<T, I>>,
}

impl<T, I: Indexer> SenderWrapper<T, I> {
    pub fn new(sender: Sender<T, I>) -> Self {
        Self {
            inner: Some(sender),
        }
    }
}

impl<T, I: Indexer> Sink<T> for SenderWrapper<T, I> {
    type Error = SendError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .as_mut()
            .map(|inner| inner.poll_ready(cx))
            .unwrap_or(Poll::Ready(Err(SendError::Disconnected)))
    }

    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.inner
            .as_mut()
            .map(|inner| inner.start_send(item))
            .unwrap_or(Err(SendError::Disconnected))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .as_mut()
            .map(|inner| inner.poll_flush(cx))
            .unwrap_or(Poll::Ready(Err(SendError::Disconnected)))
    }

    fn poll_close(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner = None;
        Poll::Ready(Ok(()))
    }
}

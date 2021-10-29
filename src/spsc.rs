use crate::atomic_waker::AtomicWaker;
use crate::error::{SendError, TrySendError};
use crate::ring::BoundedRing;
use crate::ring::Ring;
use crate::TryRecvError;
use futures_util::future::poll_fn;
use futures_util::Stream;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

struct Shared<T, R> {
    _marker: PhantomData<T>,
    ring: R,
    consumer: AtomicWaker,
    producer: AtomicWaker,
}

unsafe impl<T: Send, R> Send for Sender<T, R> {}
unsafe impl<T: Send, R> Sync for Sender<T, R> {}

unsafe impl<T: Send, R> Send for Receiver<T, R> {}
unsafe impl<T: Send, R> Sync for Receiver<T, R> {}

impl<T, R> Shared<T, R> {
    fn new(ring: R) -> Self {
        Self {
            _marker: Default::default(),
            ring,
            consumer: Default::default(),
            producer: Default::default(),
        }
    }
}

pub type BoundedSender<T> = Sender<T, BoundedRing<T>>;
pub type BoundedReceiver<T> = Receiver<T, BoundedRing<T>>;

pub fn channel<T>(size: usize) -> (BoundedSender<T>, BoundedReceiver<T>) {
    let ring = BoundedRing::with_capacity(size);
    let shared = Arc::new(Shared::new(ring));
    (
        Sender {
            inner: shared.clone(),
        },
        Receiver { inner: shared },
    )
}

pub struct Sender<T, R> {
    inner: Arc<Shared<T, R>>,
}

impl<T, R> Drop for Sender<T, R> {
    fn drop(&mut self) {
        // we need to wake up the receiver before
        // the sender was totally dropped, otherwise the receiver may hang up.
        self.inner.consumer.wake_by_ref();
    }
}

impl<T, R: Ring<T>> Sender<T, R> {
    pub fn start_send(&mut self, item: T) -> Result<(), SendError> {
        if let Some(idx) = self.inner.ring.next_idx() {
            unsafe {
                self.inner.ring.set(item, idx);
            }
            Ok(())
        } else {
            Err(SendError::Full)
        }
    }

    pub fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        if self.inner.ring.is_full() {
            self.poll_flush(cx)
        } else {
            Poll::Ready(Ok(()))
        }
    }

    pub fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        if self.is_closed() {
            Poll::Ready(Err(SendError::Disconnected))
        } else if self.inner.ring.is_empty() {
            // if the inner ring is already empty,
            // we just return ok to avoid some atomic operation.
            Poll::Ready(Ok(()))
        } else {
            self.inner.producer.register(cx.waker());
            self.inner.consumer.wake_by_ref();
            Poll::Pending
        }
    }

    pub async fn send(&mut self, item: T) -> Result<(), TrySendError<T>> {
        let idx = match poll_fn(|cx| self.poll_next_pos(cx)).await {
            Ok(idx) => idx,
            Err(err) => return Err(TrySendError { err, val: item }),
        };

        unsafe {
            self.inner.ring.set(item, idx);
        }

        self.inner.consumer.wake_by_ref();

        Ok(())
    }

    fn poll_next_pos(&mut self, cx: &mut Context<'_>) -> Poll<Result<usize, SendError>> {
        if self.is_closed() {
            return Poll::Ready(Err(SendError::Disconnected));
        }

        if let Some(idx) = self.inner.ring.next_idx() {
            Poll::Ready(Ok(idx))
        } else {
            self.inner.producer.register(cx.waker());
            Poll::Pending
        }
    }

    pub fn is_closed(&self) -> bool {
        Arc::strong_count(&self.inner) <= 1
    }
}

pub struct Receiver<T, R> {
    inner: Arc<Shared<T, R>>,
}

impl<T, R: Ring<T>> Stream for Receiver<T, R> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_recv(cx)
    }
}

impl<T, R: Ring<T>> Receiver<T, R> {
    fn poll_next_msg(&self) -> Poll<Option<T>> {
        match self.inner.ring.try_pop() {
            None => {
                if self.is_closed() {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
            Some(item) => {
                self.inner.producer.wake_by_ref();
                Poll::Ready(Some(item))
            }
        }
    }

    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        match self.inner.ring.try_pop() {
            None => {
                if self.is_closed() {
                    Err(TryRecvError::Disconnected)
                } else {
                    Err(TryRecvError::Empty)
                }
            }
            Some(item) => Ok(item),
        }
    }

    pub fn poll_want_recv(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.is_closed() {
            return Poll::Ready(());
        }

        self.inner.consumer.register(cx.waker());
        self.inner.producer.wake_by_ref();
        if self.inner.ring.is_empty() {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }

    pub async fn want_recv(&mut self) {
        poll_fn(|cx| self.poll_want_recv(cx)).await
    }

    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        if let Poll::Ready(op) = self.poll_next_msg() {
            return Poll::Ready(op);
        }

        self.inner.consumer.register(cx.waker());

        // poll again, in case of some item was sent between the registering and the previous poll.
        self.poll_next_msg()
    }

    pub async fn recv(&mut self) -> Option<T> {
        poll_fn(|cx| self.poll_recv(cx)).await
    }

    pub fn is_closed(&self) -> bool {
        Arc::strong_count(&self.inner) <= 1
    }
}

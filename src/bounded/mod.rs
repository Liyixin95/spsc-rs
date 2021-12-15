mod ring;
pub mod wrapper;

use crate::atomic_waker::AtomicWaker;
use crate::bounded::ring::{And, ExactRing, Indexer, P2Ring, Remainder, Ring};
use crate::error::TryRecvError;
use crate::error::{SendError, TrySendError};
use crate::loom::{Arc, AtomicBool, Ordering};
use futures_util::future::poll_fn;
use futures_util::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Shared<T, I: Indexer> {
    ring: Ring<T, I>,
    consumer: AtomicWaker,
    producer: AtomicWaker,
    closed: AtomicBool,
}

impl<T, I: Indexer> Shared<T, I> {
    fn new(ring: Ring<T, I>) -> Self {
        Self {
            ring,
            consumer: Default::default(),
            producer: Default::default(),
            closed: Default::default(),
        }
    }
}

pub type P2Sender<T> = Sender<T, And>;
pub type P2Receiver<T> = Receiver<T, And>;

pub fn channel<T>(size: usize) -> (P2Sender<T>, P2Receiver<T>) {
    let ring = P2Ring::with_capacity(size);
    let shared = Arc::new(Shared::new(ring));
    (
        Sender {
            inner: shared.clone(),
        },
        Receiver { inner: shared },
    )
}

pub type ExactSender<T> = Sender<T, Remainder>;
pub type ExactReceiver<T> = Receiver<T, Remainder>;

pub fn exact_channel<T>(size: usize) -> (ExactSender<T>, ExactReceiver<T>) {
    let ring = ExactRing::with_capacity(size);
    let shared = Arc::new(Shared::new(ring));
    (
        Sender {
            inner: shared.clone(),
        },
        Receiver { inner: shared },
    )
}

pub struct Sender<T, I: Indexer> {
    inner: Arc<Shared<T, I>>,
}

impl<T, I: Indexer> Drop for Sender<T, I> {
    fn drop(&mut self) {
        // we need to wake up the receiver before
        // the sender was totally dropped, otherwise the receiver may hang up.
        self.inner.closed.store(true, Ordering::Release);
        self.inner.consumer.wake_by_ref();
    }
}

impl<T, I: Indexer> Sender<T, I> {
    pub fn start_send(&mut self, item: T) -> Result<(), SendError> {
        if let Some(idx) = self.inner.ring.next_idx() {
            unsafe {
                self.inner.ring.set_unchecked(item, idx);
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
            // if the inner bounded is already empty,
            // we just return ok to avoid some atomic operation.
            Poll::Ready(Ok(()))
        } else {
            self.inner.producer.register(cx.waker());
            self.inner.consumer.wake_by_ref();
            Poll::Pending
        }
    }

    pub async fn flush(&mut self) -> Result<(), SendError> {
        poll_fn(|cx| self.poll_flush(cx)).await
    }

    pub async fn send(&mut self, item: T) -> Result<(), TrySendError<T>> {
        let idx = match poll_fn(|cx| self.poll_next_pos(cx)).await {
            Ok(idx) => idx,
            Err(err) => return Err(TrySendError { err, val: item }),
        };

        unsafe {
            self.inner.ring.set_unchecked(item, idx);
        }

        self.inner.consumer.wake_by_ref();

        Ok(())
    }

    /// Returns whether this channel is closed.
    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    fn poll_next_pos(&mut self, cx: &mut Context<'_>) -> Poll<Result<usize, SendError>> {
        if self.is_closed() {
            return Poll::Ready(Err(SendError::Disconnected));
        }

        if let Some(idx) = self.inner.ring.next_idx() {
            Poll::Ready(Ok(idx))
        } else {
            self.inner.producer.register(cx.waker());

            // We need to poll again, in case of the receiver take some items during
            // the register and the previous poll
            if let Some(idx) = self.inner.ring.next_idx() {
                Poll::Ready(Ok(idx))
            } else {
                Poll::Pending
            }
        }
    }
}

pub struct Receiver<T, I: Indexer> {
    inner: Arc<Shared<T, I>>,
}

impl<T, I: Indexer> Stream for Receiver<T, I> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_recv(cx)
    }
}

impl<T, I: Indexer> Receiver<T, I> {
    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        match self.inner.ring.try_pop() {
            None => {
                // If there is no item in this bounded, we need to
                // check closed and try pop again.
                //
                // Consider this situation:
                // receiver try pop first, and sender send an item then close.
                // If we just check closed without pop again, the remaining item will be lost.
                if self.is_closed() {
                    match self.inner.ring.try_pop() {
                        None => Err(TryRecvError::Disconnected),
                        Some(item) => Ok(item),
                    }
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
            return Poll::Ready(Some(op));
        }

        self.inner.consumer.register(cx.waker());

        // 1. We need to poll again,
        //    in case of some item was sent between the registering and the previous poll.
        //
        // 2. We need to see whether this channel is closed. Because the sender could
        //    be closed and wake receiver before the register operation, so if we don't check close,
        //    this method may return Pending and will never be wakeup.
        if self.is_closed() {
            match self.poll_next_msg() {
                Poll::Ready(op) => Poll::Ready(Some(op)),
                Poll::Pending => Poll::Ready(None),
            }
        } else {
            self.poll_next_msg().map(Some)
        }
    }

    pub async fn recv(&mut self) -> Option<T> {
        poll_fn(|cx| self.poll_recv(cx)).await
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub fn close(&mut self) {
        self.inner.closed.store(true, Ordering::Release)
    }

    fn poll_next_msg(&self) -> Poll<T> {
        match self.inner.ring.try_pop() {
            None => Poll::Pending,
            Some(item) => {
                self.inner.producer.wake_by_ref();
                Poll::Ready(item)
            }
        }
    }
}

use crate::atomic_waker::AtomicWaker;
use crate::error::{SendError, TrySendError};
use crate::ring::BoundedRing;
use crate::ring::Ring;
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

pub type ArraySender<T> = Sender<T, BoundedRing<T>>;
pub type ArrayReceiver<T> = Receiver<T, BoundedRing<T>>;

pub fn channel<T>(size: usize) -> (Sender<T, BoundedRing<T>>, Receiver<T, BoundedRing<T>>) {
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
        self.inner.consumer.wake_by_ref();
    }
}

impl<T, R: Ring<T>> Sender<T, R> {
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

    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        if let Poll::Ready(op) = self.poll_next_msg() {
            return Poll::Ready(op);
        }

        self.inner.consumer.register(cx.waker());

        self.poll_next_msg()
    }

    pub async fn recv(&mut self) -> Option<T> {
        poll_fn(|cx| self.poll_recv(cx)).await
    }

    pub fn is_closed(&self) -> bool {
        Arc::strong_count(&self.inner) <= 1
    }
}

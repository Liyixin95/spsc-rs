use crate::atomic_waker::AtomicWaker;
use crate::error::{SendError, TryRecvError, TrySendError};
use crate::loom::{Arc, AtomicBool, Ordering};
use crate::unbounded::queue::Queue;
use futures_util::future::poll_fn;
use futures_util::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

mod queue;
pub mod wrapper;

pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    let shared = Shared {
        queue: Queue::new(),
        consumer: AtomicWaker::default(),
        closed: AtomicBool::new(false),
    };
    let ptr = Arc::new(shared);
    (
        UnboundedSender { inner: ptr.clone() },
        UnboundedReceiver { inner: ptr },
    )
}

struct Shared<T> {
    queue: Queue<T>,
    consumer: AtomicWaker,
    closed: AtomicBool,
}

pub struct UnboundedSender<T> {
    inner: Arc<Shared<T>>,
}

impl<T> Drop for UnboundedSender<T> {
    fn drop(&mut self) {
        // we need to wake up the receiver before
        // the sender was totally dropped, otherwise the receiver may hang up.
        self.inner.closed.store(true, Ordering::Release);
        self.inner.consumer.wake_by_ref();
    }
}

impl<T> UnboundedSender<T> {
    pub fn start_send(&mut self, t: T) -> Result<(), SendError> {
        if self.is_closed() {
            Err(SendError::Disconnected)
        } else {
            self.push(t);
            Ok(())
        }
    }

    pub fn send(&mut self, t: T) -> Result<(), TrySendError<T>> {
        if self.is_closed() {
            Err(TrySendError {
                err: SendError::Disconnected,
                val: t,
            })
        } else {
            self.push(t);
            self.inner.consumer.wake_by_ref();
            Ok(())
        }
    }

    pub fn flush(&mut self) -> Result<(), SendError> {
        if self.is_closed() {
            Err(SendError::Disconnected)
        } else if self.inner.queue.is_empty() {
            // if the inner queue is already empty,
            // we just return ok to avoid some atomic operation.
            Ok(())
        } else {
            self.inner.consumer.wake_by_ref();
            Ok(())
        }
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub fn close(&mut self) {
        self.inner.closed.store(true, Ordering::Release)
    }

    fn push(&mut self, t: T) {
        // Safety: The sender can not be cloned, and take mut reference.
        // So there would only exist one sender, which means we can
        // safely push to the inner queue.
        unsafe { self.inner.queue.push(t) }
    }
}

pub struct UnboundedReceiver<T> {
    inner: Arc<Shared<T>>,
}

impl<T> Stream for UnboundedReceiver<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_recv(cx)
    }
}

impl<T> UnboundedReceiver<T> {
    pub async fn receive(&mut self) -> Option<T> {
        poll_fn(|cx| self.poll_recv(cx)).await
    }

    pub fn poll_want_recv(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.is_closed() {
            return Poll::Ready(());
        }

        self.inner.consumer.register(cx.waker());
        if self.inner.queue.is_empty() {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }

    pub async fn want_recv(&mut self) {
        poll_fn(|cx| self.poll_want_recv(cx)).await
    }

    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        match self.try_pop() {
            None => {
                // If there is no item in this bounded, we need to
                // check closed and try pop again.
                //
                // Consider this situation:
                // receiver try pop first, and sender send an item then close.
                // If we just check closed without pop again, the remaining item will be lost.
                if self.is_closed() {
                    match self.try_pop() {
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

    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        if let Some(item) = self.try_pop() {
            return Poll::Ready(Some(item));
        }

        self.inner.consumer.register(cx.waker());

        // 1. We need to poll again,
        //    in case of some item was sent between the registering and the previous poll.
        //
        // 2. We need to see whether this channel is closed. Because the sender could
        //    be closed and wake receiver before the register operation, so if we don't check close,
        //    this method may return Pending and will never be wakeup.
        if self.is_closed() {
            Poll::Ready(self.try_pop())
        } else {
            match self.try_pop() {
                None => Poll::Pending,
                Some(item) => Poll::Ready(Some(item)),
            }
        }
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub fn close(&mut self) {
        self.inner.closed.store(true, Ordering::Release)
    }

    fn try_pop(&mut self) -> Option<T> {
        // Safety: The receiver can not be cloned, and take mut reference.
        // So there would only exist one receiver, which means we can
        // safely pop from the inner queue.
        unsafe { self.inner.queue.try_pop() }
    }
}

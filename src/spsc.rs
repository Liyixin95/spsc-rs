use std::sync::Arc;
use crate::raw_ring::SimpleRing;
use futures::task::AtomicWaker;
use std::task::{Poll, Context};
use std::cell::UnsafeCell;

struct Shared<T> {
    ring: UnsafeCell<SimpleRing<T>>,
    consumer: AtomicWaker,
    producer: AtomicWaker,
}

unsafe impl<T: Send> Send for Shared<T> {}
unsafe impl<T: Send> Sync for Shared<T> {}

impl<T> Shared<T> {
    fn new(size: usize) -> Self {
        Self {
            ring: UnsafeCell::new(SimpleRing::with_capacity(size)),
            consumer: Default::default(),
            producer: Default::default(),
        }
    }
}

pub fn channel<T>(size: usize) -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared::new(size));
    (Sender {
        inner: shared.clone()
    }, Receiver {
        inner: shared
    })
}

pub struct Sender<T> {
    inner: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    pub fn poll_send(&mut self, t: T, cx: &mut Context<'_>) -> Poll<Result<(), ()>> {
        if Arc::strong_count(&self.inner) <= 1 {
            return Poll::Ready(Err(()));
        }

        unsafe {
            let ring = &mut *self.inner.ring.get();
            if let Some(next) = ring.next() {
                ring.set(next, t);
                self.inner.consumer.wake();
                Poll::Ready(Ok(()))
            } else {
                self.inner.producer.register(cx.waker());
                Poll::Pending
            }
        }
    }
}

pub struct Receiver<T> {
    inner: Arc<Shared<T>>,
}

impl<T> Receiver<T> {
    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        if Arc::strong_count(&self.inner) <= 1 {
            return Poll::Ready(None);
        }

        unsafe {
            let ring = &mut *self.inner.ring.get();
            if let Some(t) = ring.try_pop() {
                self.inner.producer.wake();
                Poll::Ready(Some(t))
            } else {
                self.inner.consumer.register(cx.waker());
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::join;
    use futures::future::poll_fn;
    use tokio::time::timeout;
    use std::time::Duration;

    #[tokio::test]
    async fn send_without_wake() {
        let (mut tx, mut rx) = channel(64);
        std::mem::forget(rx);

        for _ in 0..64 {
            poll_fn(|cx| tx.poll_send(1, cx)).await;
        }
    }

    async fn test() {
        let (mut tx, mut rx) = channel(64);

        let handle = tokio::spawn(async move {
            for i in 0..1024 {
                poll_fn(|cx| tx.poll_send(i, cx)).await;
            }
        });
        let handle1 = tokio::spawn(async move {
            while let Some(i) = poll_fn(|cx| rx.poll_recv(cx)).await {}
        });

        timeout(Duration::from_secs(1), join(handle, handle1)).await.unwrap();
    }

    #[tokio::test]
    async fn multi_test() {
        for _ in 0..1000 {
            test().await;
        }
    }
}
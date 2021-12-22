use futures_util::SinkExt;
use spsc_rs::error::TryRecvError;
use spsc_rs::SenderWrapper;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{hint, thread};
use tokio::runtime::Builder;

fn block_on<F: Future>(f: F) -> F::Output {
    let mut builder = Builder::new_current_thread();
    let rt = builder.build().unwrap();

    rt.block_on(f)
}

fn receive_test_framework<R, F, Fut1, Fut2>(amt: u32, cap: usize, sender: F, receiver: R)
where
    F: FnOnce(u32, spsc_rs::P2Sender<u32>) -> Fut1 + Send + 'static,
    Fut1: Future + Send + 'static,
    Fut1::Output: Send,
    R: FnOnce(u32, spsc_rs::P2Receiver<u32>) -> Fut2 + Send + 'static,
    Fut2: Future + Send + 'static,
    Fut2::Output: Send,
{
    let (tx, rx) = spsc_rs::channel(cap);

    let tx_complete = Arc::new(AtomicBool::new(false));
    let tx_complete1 = tx_complete.clone();

    let t = thread::spawn(move || {
        let ret = block_on(sender(amt, tx));
        tx_complete1.store(true, Ordering::Release);
        ret
    });

    let rx_complete = Arc::new(AtomicBool::new(false));
    let rx_complete1 = rx_complete.clone();

    let t1 = thread::spawn(move || {
        let ret = block_on(receiver(amt, rx));
        rx_complete1.store(true, Ordering::Release);
        ret
    });

    let now = Instant::now();
    loop {
        if tx_complete.load(Ordering::Acquire) && rx_complete.load(Ordering::Acquire) {
            let _ = t.join().unwrap();
            let _ = t1.join().unwrap();
            return;
        }

        if now.elapsed() > Duration::from_secs(3) {
            panic!("exec timeout")
        }

        hint::spin_loop();
    }
}

async fn receive_sequence(amt: u32, mut rx: spsc_rs::P2Receiver<u32>) {
    let mut n = 0;
    while let Some(i) = rx.recv().await {
        assert_eq!(i, n);
        n += 1;
    }

    assert_eq!(n, amt);
}

async fn try_receive_sequence(amt: u32, mut rx: spsc_rs::P2Receiver<u32>) {
    let mut n = 0;
    loop {
        match rx.try_recv() {
            Ok(i) => {
                assert_eq!(i, n);
                n += 1;
            }
            Err(TryRecvError::Empty) => rx.want_recv().await,
            Err(TryRecvError::Disconnected) => break,
        }
    }
    assert_eq!(n, amt);
}

async fn batch_sequence(n: u32, sender: spsc_rs::P2Sender<u32>) {
    let mut sink = SenderWrapper::new(sender);
    for x in 0..n {
        sink.feed(x).await.unwrap();
    }
}

async fn send_sequence(n: u32, mut sender: spsc_rs::P2Sender<u32>) {
    for x in 0..n {
        sender.send(x).await.unwrap();
    }
}

const COUNT: usize = 1000;

#[test]
#[cfg_attr(miri, ignore)]
fn batch_test() {
    // batch send
    for _ in 0..COUNT {
        receive_test_framework(10000, 2, batch_sequence, receive_sequence);
    }

    for _ in 0..COUNT {
        receive_test_framework(10000, 100, batch_sequence, receive_sequence);
    }

    // batch send and batch receive
    for _ in 0..COUNT {
        receive_test_framework(10000, 2, batch_sequence, try_receive_sequence);
    }

    for _ in 0..COUNT {
        receive_test_framework(10000, 100, batch_sequence, try_receive_sequence);
    }

    // batch receive
    for _ in 0..COUNT {
        receive_test_framework(10000, 2, send_sequence, try_receive_sequence);
    }

    for _ in 0..COUNT {
        receive_test_framework(10000, 100, send_sequence, try_receive_sequence);
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn spsc_test() {
    for _ in 0..COUNT {
        receive_test_framework(10000, 2, send_sequence, receive_sequence);
    }

    for _ in 0..COUNT {
        receive_test_framework(10000, 100, send_sequence, receive_sequence);
    }
}

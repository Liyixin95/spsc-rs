use futures_executor::block_on;
use futures_util::SinkExt;
use spsc_rs::wrapper::SenderWrapper;
use spsc_rs::{spsc, TryRecvError};
use std::future::Future;
use std::thread;

fn receive_test_framework<R, F, Fut1, Fut2>(amt: u32, cap: usize, sender: F, receiver: R)
where
    F: FnOnce(u32, spsc::BoundedSender<u32>) -> Fut1 + Send + 'static,
    Fut1: Future + Send + 'static,
    Fut1::Output: Send,
    R: FnOnce(u32, spsc::BoundedReceiver<u32>) -> Fut2 + Send + 'static,
    Fut2: Future + Send + 'static,
    Fut2::Output: Send,
{
    let (tx, rx) = spsc::channel(cap);
    let t = thread::spawn(move || {
        //tokio's runtime can not be used in miri
        block_on(sender(amt, tx))
    });

    block_on(receiver(amt, rx));

    t.join().unwrap();
}

async fn receive_sequence(amt: u32, mut rx: spsc::BoundedReceiver<u32>) {
    let mut n = 0;
    while let Some(i) = rx.recv().await {
        assert_eq!(i, n);
        n += 1;
    }

    assert_eq!(n, amt);
}

async fn try_receive_sequence(amt: u32, mut rx: spsc::BoundedReceiver<u32>) {
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
    println!("try receive finish");
}

async fn batch_sequence(n: u32, sender: spsc::BoundedSender<u32>) {
    let mut sink = SenderWrapper::new(sender);
    for x in 0..n {
        sink.feed(x).await.unwrap();
    }
    println!("batch test iteration finish");
}

async fn send_sequence(n: u32, mut sender: spsc::BoundedSender<u32>) {
    for x in 0..n {
        sender.send(x).await.unwrap();
    }
    println!("send test iteration finish");
}

const COUNT: usize = 100;

#[test]
fn batch_test() {
    // for _ in 0..COUNT {
    //     receive_test_framework(10000, 2, batch_sequence, receive_sequence);
    // }
    //
    // for _ in 0..COUNT {
    //     receive_test_framework(10000, 100, batch_sequence, receive_sequence);
    // }

    for _ in 0..COUNT {
        receive_test_framework(10000, 2, batch_sequence, try_receive_sequence);
    }

    for _ in 0..COUNT {
        receive_test_framework(10000, 100, batch_sequence, try_receive_sequence);
    }
}

#[test]
fn spsc_test() {
    for _ in 0..COUNT {
        receive_test_framework(10000, 2, send_sequence, receive_sequence);
    }

    for _ in 0..COUNT {
        receive_test_framework(10000, 100, send_sequence, receive_sequence);
    }
}

use futures_executor::block_on;
use futures_util::SinkExt;
use spsc_rs::spsc;
use spsc_rs::wrapper::SenderWrapper;
use std::future::Future;
use std::thread;

fn seq_test<F, Fut>(amt: u32, cap: usize, f: F)
where
    F: FnOnce(u32, spsc::BoundedSender<u32>) -> Fut + Send + 'static,
    Fut: Future + Send + 'static,
    Fut::Output: Send,
{
    let (tx, mut rx) = spsc::channel(cap);
    let t = thread::spawn(move || {
        //tokio's runtime can not be used in miri
        block_on(f(amt, tx))
    });

    block_on(async move {
        let mut n = 0;
        while let Some(i) = rx.recv().await {
            assert_eq!(i, n);
            n += 1;
        }

        assert_eq!(n, amt);
    });

    t.join().unwrap();
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

#[test]
fn spsc_test() {
    const COUNT: usize = 100;

    for _ in 0..COUNT {
        seq_test(10000, 2, send_sequence);
    }

    for _ in 0..COUNT {
        seq_test(10000, 100, send_sequence);
    }

    for _ in 0..COUNT {
        seq_test(10000, 2, batch_sequence);
    }

    for _ in 0..COUNT {
        seq_test(10000, 100, batch_sequence);
    }
}

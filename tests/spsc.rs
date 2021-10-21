use futures_channel::mpsc;
use futures_executor::block_on;
use futures_util::future::poll_fn;
use futures_util::StreamExt;
use ring_rs::spsc;
use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use tokio::runtime::{Builder, Runtime};

fn seq_test(amt: u32, cap: usize) {
    let (tx, rx) = spsc::channel(cap);
    let t = thread::spawn(move || {
        //let runtime = Builder::new_current_thread().build().unwrap();
        block_on(send_sequence(amt, tx))
    });

    //let runtime = Builder::new_current_thread().build().unwrap();
    block_on(async move {
        let mut n = 0;
        let rx = rx;
        while let Some(i) = poll_fn(|cx| rx.poll_recv(cx)).await {
            eprintln!("i = {:#?}", i);
            assert_eq!(i, n);
            n += 1;
        }

        assert_eq!(n, amt);
    });

    t.join().unwrap();
}

async fn send_sequence(n: u32, mut sender: spsc::Sender<u32>) {
    for x in 0..n {
        sender.send(x).await.unwrap();
    }
    println!("complete")
}

#[test]
fn spsc_test() {
    const COUNT: usize = 100;

    for _ in 0..COUNT {
        seq_test(10000, 2);
    }

    for _ in 0..COUNT {
        seq_test(10000, 100);
    }
}

// #[test]
// fn race_test() {
//     let i = Arc::new(AtomicUsize::new(0));
//
//     let i1 = i.clone();
//     let t1 = std::thread::spawn(move || {
//         for idx in 0..11111 {
//             i1.store(idx, Ordering::Relaxed);
//         };
//     });
//
//     let t2 = std::thread::spawn(move || {
//         for _ in 0..11111 {
//             let _ = i.load(Ordering::Relaxed);
//         };
//     });
//
//     t1.join();
//     t2.join();
// }

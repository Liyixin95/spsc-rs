use futures_executor::block_on;
use spsc_rs::spsc;
use std::thread;

fn seq_test(amt: u32, cap: usize) {
    let (tx, mut rx) = spsc::channel(cap);
    let t = thread::spawn(move || {
        //tokio's runtime can not be used in miri
        block_on(send_sequence(amt, tx))
    });

    block_on(async move {
        let mut n = 0;
        while let Some(i) = rx.recv().await {
            eprintln!("i = {:#?}", i);
            assert_eq!(i, n);
            n += 1;
        }

        assert_eq!(n, amt);
    });

    t.join().unwrap();
}

async fn send_sequence(n: u32, mut sender: spsc::BoundedSender<u32>) {
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

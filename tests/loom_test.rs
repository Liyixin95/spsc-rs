#![cfg(loom)]

use loom::future::block_on;
use loom::thread;
use spsc_rs::error::TryRecvError;

#[test]
fn bounded_send_try_recv() {
    loom::model(|| {
        let (mut tx, mut rx) = spsc_rs::channel(1);
        thread::spawn(move || {
            block_on(async move {
                tx.send(0).await.unwrap();
                tx.send(1).await.unwrap();
            })
        });

        block_on(async move {
            let mut count = 0;
            loop {
                match rx.try_recv() {
                    Ok(idx) => {
                        assert_eq!(idx, count);
                        count += 1;
                    }
                    Err(TryRecvError::Empty) => rx.want_recv().await,
                    Err(TryRecvError::Disconnected) => break,
                }

                loom::thread::yield_now();
            }

            assert_eq!(count, 2);
        })
    })
}

#[test]
fn bounded_send_receive() {
    loom::model(|| {
        let (mut tx, mut rx) = spsc_rs::channel(1);

        thread::spawn(move || {
            block_on(async move {
                tx.send(0).await.unwrap();
                tx.send(1).await.unwrap();
            })
        });

        block_on(async move {
            let idx = rx.recv().await.unwrap();
            assert_eq!(idx, 0);

            let idx = rx.recv().await.unwrap();
            assert_eq!(idx, 1);
        })
    })
}

#[test]
fn bounded_closing_tx() {
    loom::model(|| {
        let (mut tx, mut rx) = spsc_rs::channel(16);

        thread::spawn(move || {
            tx.start_send(()).unwrap();
            drop(tx);
        });

        let v = block_on(rx.recv());
        assert!(v.is_some());

        let v = block_on(rx.recv());
        assert!(v.is_none());
    });
}

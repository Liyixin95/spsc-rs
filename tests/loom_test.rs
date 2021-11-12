#![cfg(loom)]

use loom::future::block_on;
use loom::thread;
use spsc_rs::spsc;

#[test]
fn send_receive() {
    loom::model(|| {
        let (mut tx, mut rx) = spsc::channel(1);

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
fn closing_tx() {
    loom::model(|| {
        let (mut tx, mut rx) = spsc::channel(16);

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

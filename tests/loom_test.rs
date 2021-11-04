#![cfg(loom)]

use loom::future::block_on;
use loom::thread;
use spsc_rs::spsc;

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

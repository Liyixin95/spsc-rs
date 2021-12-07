#[test]
fn bounded_drop_test() {
    let (mut tx, rx) = spsc_rs::channel(64);

    tx.start_send([1; 1024]).unwrap();
    tx.start_send([1; 1024]).unwrap();
    tx.start_send([1; 1024]).unwrap();

    std::mem::drop(rx);
}

#[test]
fn bounded_send_receive() {
    let (mut tx, mut rx) = spsc_rs::channel(64);

    tx.start_send([1; 1024]).unwrap();
    tx.start_send([1; 1024]).unwrap();
    tx.start_send([1; 1024]).unwrap();

    let _ = rx.try_recv().unwrap();
    let _ = rx.try_recv().unwrap();
    let _ = rx.try_recv().unwrap();
}

#[test]
fn unbounded_drop_test() {
    // one block in this channel
    let (mut tx, rx) = spsc_rs::unbounded_channel();

    tx.send([1; 1024]).unwrap();
    tx.send([1; 1024]).unwrap();
    tx.send([1; 1024]).unwrap();

    std::mem::drop(rx);

    // multi block in this channel
    let (mut tx, rx) = spsc_rs::unbounded_channel();

    for _ in 0..100 {
        tx.send([1; 64]).unwrap();
    }

    std::mem::drop(rx);
}

#[test]
fn unbounded_send_receive() {
    // one block in this channel
    let (mut tx, mut rx) = spsc_rs::unbounded_channel();

    tx.send([1; 1024]).unwrap();
    tx.send([1; 1024]).unwrap();
    tx.send([1; 1024]).unwrap();

    let _ = rx.try_receive().unwrap();
    let _ = rx.try_receive().unwrap();
    let _ = rx.try_receive().unwrap();

    // multi block in this channel
    let (mut tx, mut rx) = spsc_rs::unbounded_channel();

    for _ in 0..100 {
        tx.send([1; 64]).unwrap();
    }

    for _ in 0..100 {
        let _ = rx.try_receive().unwrap();
    }
}

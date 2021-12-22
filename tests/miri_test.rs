
use spsc_rs::error::SendError;


#[test]
fn bounded_drop_test() {
    let (mut tx, rx) = spsc_rs::channel(64);

    tx.start_send([1; 8]).unwrap();
    tx.start_send([1; 8]).unwrap();
    tx.start_send([1; 8]).unwrap();

    std::mem::drop(rx);
}

#[test]
fn bounded_send_receive() {
    let (mut tx, mut rx) = spsc_rs::channel(64);

    tx.start_send([1; 8]).unwrap();
    tx.start_send([1; 8]).unwrap();
    tx.start_send([1; 8]).unwrap();

    let _ = rx.try_recv().unwrap();
    let _ = rx.try_recv().unwrap();
    let _ = rx.try_recv().unwrap();
}

#[test]
fn bounded_close_test() {
    let (mut tx, mut rx) = spsc_rs::channel(64);

    tx.start_send([1; 8]).unwrap();
    tx.start_send([1; 8]).unwrap();
    tx.start_send([1; 8]).unwrap();

    rx.close();

    matches!(tx.start_send([1; 8]), Err(SendError::Disconnected));

    let _ = rx.try_recv().unwrap();
    let _ = rx.try_recv().unwrap();
    let _ = rx.try_recv().unwrap();
}

#[test]
fn unbounded_drop_test() {
    // one block in this channel
    let (mut tx, rx) = spsc_rs::unbounded_channel();

    tx.send([1; 8]).unwrap();
    tx.send([1; 8]).unwrap();
    tx.send([1; 8]).unwrap();

    std::mem::drop(rx);

    // multi block in this channel
    let (mut tx, rx) = spsc_rs::unbounded_channel();

    for _ in 0..100 {
        tx.send([1; 8]).unwrap();
    }

    std::mem::drop(rx);
}

#[test]
fn unbounded_send_receive() {
    // one block in this channel
    let (mut tx, mut rx) = spsc_rs::unbounded_channel();

    tx.send([1; 8]).unwrap();
    tx.send([1; 8]).unwrap();
    tx.send([1; 8]).unwrap();

    let _ = rx.try_recv().unwrap();
    let _ = rx.try_recv().unwrap();
    let _ = rx.try_recv().unwrap();

    // multi block in this channel
    let (mut tx, mut rx) = spsc_rs::unbounded_channel();

    for _ in 0..100 {
        tx.send([1; 8]).unwrap();
    }

    for _ in 0..100 {
        let _ = rx.try_recv().unwrap();
    }
}

#[test]
fn unbounded_close_test() {
    let (mut tx, mut rx) = spsc_rs::unbounded_channel();

    tx.start_send([1; 8]).unwrap();
    tx.start_send([1; 8]).unwrap();
    tx.start_send([1; 8]).unwrap();

    rx.close();

    matches!(tx.start_send([1; 8]), Err(SendError::Disconnected));

    let _ = rx.try_recv().unwrap();
    let _ = rx.try_recv().unwrap();
    let _ = rx.try_recv().unwrap();
}

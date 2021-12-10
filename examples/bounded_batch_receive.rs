use spsc_rs::error::TryRecvError;

#[tokio::main]
async fn main() {
    let (mut tx, mut rx) = spsc_rs::channel(128);

    tokio::spawn(async move {
        for i in 0..1024 {
            tx.send(i).await.unwrap();
        }
    });

    loop {
        match rx.try_recv() {
            Ok(i) => println!("got = {}", i),
            Err(TryRecvError::Empty) => rx.want_recv().await,
            Err(TryRecvError::Disconnected) => break,
        }
    }
}

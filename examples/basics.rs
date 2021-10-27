use spsc_rs::spsc;

#[tokio::main]
async fn main() {
    let (mut tx, mut rx) = spsc::channel(128);

    tokio::spawn(async move {
        for i in 0..10 {
            if tx.send(i).await.is_err() {
                println!("receiver dropped");
                return;
            }
        }
    });

    while let Some(i) = rx.recv().await {
        println!("got = {}", i);
    }
}

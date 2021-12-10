use spsc_rs::error::SendError;

#[tokio::main]
async fn main() {
    let (mut tx, mut rx) = spsc_rs::channel(128);

    tokio::spawn(async move {
        for i in 0..1024 {
            match tx.start_send(i) {
                Ok(_) => {}
                Err(SendError::Full) => tx.flush().await?,
                Err(e) => return Err(e),
            }
        }

        Ok(())
    });

    while let Some(i) = rx.recv().await {
        println!("got = {}", i);
    }
}

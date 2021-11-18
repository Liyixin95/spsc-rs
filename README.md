# Async SPSC Channel

![action badge](https://github.com/Liyixin95/spsc-rs/actions/workflows/ci.yml/badge.svg)

---

Wait-free(even without CAS), async, single-producer single-consumer channel.

# Examples
The usage of this channel is almost the same as `tokio`'s channel.

```rust
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
```

# License
Licensed under either of

- Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)
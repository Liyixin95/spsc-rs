use criterion::{black_box, criterion_group, criterion_main, Criterion};
use criterion::async_executor::FuturesExecutor;
use ring_rs::spsc as ring_spsc;
use tokio::sync::mpsc as tokio_mpsc;
use futures::future::poll_fn;

const SIZE: usize = 32;

async fn bench_ring() {
    let (mut tx, rx) = ring_spsc::channel(SIZE);
    for i in 0..SIZE {
        poll_fn(|cx| tx.poll_send(i, cx)).await;
    }

    std::mem::drop(rx)
}

pub fn ring_spsc(c: &mut Criterion) {
    c.bench_function("ring spsc", |b| b.to_async(FuturesExecutor).iter(bench_ring));
}

async fn bench_tokio() {
    let (tx, rx) = tokio_mpsc::channel(SIZE);
    for i in 0..SIZE {
        tx.send(i).await;
    }

    std::mem::drop(rx)
}

pub fn tokio_channel(c: &mut Criterion) {
    c.bench_function("tokio channel", |b| b.to_async(FuturesExecutor).iter(bench_tokio));
}

criterion_group!(benches, ring_spsc, tokio_channel);
criterion_main!(benches);
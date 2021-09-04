use criterion::{black_box, criterion_group, criterion_main, Criterion};
use criterion::async_executor::FuturesExecutor;
use ring_rs::spsc as ring_spsc;
use tokio::sync::mpsc as tokio_mpsc;
use futures::future::poll_fn;
use ring_rs::raw_ring::SimpleRing;
use ring_rs::ptr_ring::PtrRing;

const SIZE: usize = 32;

fn bench_simple() {
    let mut ring = SimpleRing::<usize>::with_capacity(SIZE);
    while let Some(next) = ring.next() {
        black_box(next);
    }
}

pub fn simple_ring(c: &mut Criterion) {
    c.bench_function("simple", |b| b.iter(bench_simple));
}

fn bench_tokio() {
    let mut ring = PtrRing::<usize>::with_capacity(SIZE);
    while let Some(next) = ring.next() {
        black_box(next);
    }
}

pub fn tagged_ring(c: &mut Criterion) {
    let mut ring = PtrRing::<usize>::with_capacity(SIZE);
    c.bench_function("tagged ring", |b| b.iter(bench_tokio));
}

criterion_group!(benches, simple_ring, tagged_ring);
criterion_main!(benches);
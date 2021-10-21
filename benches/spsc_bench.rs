use criterion::{criterion_group, criterion_main, Criterion};
use spsc_rs::spsc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc as tokio_mpsc;

fn rt() -> Runtime {
    tokio::runtime::Builder::new_multi_thread().build().unwrap()
}

fn no_contention_spsc(c: &mut Criterion) {
    let rt = rt();
    c.bench_function("ring spsc", |b| {
        b.iter(|| {
            let (mut tx, mut rx) = spsc::channel(4096);

            let _ = rt.block_on(async move {
                for i in 0..4096 {
                    tx.send(i).await.unwrap();
                }
            });

            let _ = rt.block_on(async move {
                for _ in 0..4096 {
                    rx.recv().await;
                }
            });
        })
    });
}

fn no_contention_mpsc(c: &mut Criterion) {
    let rt = rt();
    c.bench_function("tokio channel", |b| {
        b.iter(|| {
            let (tx, mut rx) = tokio_mpsc::channel(4096);

            let _ = rt.block_on(async move {
                for i in 0..4096 {
                    tx.send(i).await.unwrap();
                }
            });

            let _ = rt.block_on(async move {
                for _ in 0..4096 {
                    rx.recv().await;
                }
            });
        })
    });
}

fn contention_spsc(c: &mut Criterion) {
    c.bench_function("contention ring spsc", |b| {
        b.to_async(rt()).iter(|| async move {
            let (mut tx, mut rx) = spsc::channel(4096);

            tokio::spawn(async move {
                for i in 0..4096 {
                    tx.send(i).await.unwrap();
                }
            });

            for _ in 0..4096 {
                rx.recv().await;
            }
        })
    });
}

fn contention_mpsc(c: &mut Criterion) {
    c.bench_function("contention mpsc", |b| {
        b.to_async(rt()).iter(|| async move {
            let (tx, mut rx) = tokio_mpsc::channel(4096);

            tokio::spawn(async move {
                for i in 0..4096 {
                    tx.send(i).await.unwrap();
                }
            });

            for _ in 0..4096 {
                rx.recv().await;
            }
        })
    });
}

criterion_group!(uncontention, no_contention_spsc, no_contention_mpsc);
criterion_group!(contention, contention_spsc, contention_mpsc);
criterion_main!(uncontention, contention);

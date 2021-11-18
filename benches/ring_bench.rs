use criterion::{criterion_group, criterion_main, Criterion};

fn exact_ring(c: &mut Criterion) {
    c.bench_function("exact ring", |b| {
        b.iter(|| async move {
            let (mut tx, rx) = spsc_rs::exact_channel(65536);

            std::mem::forget(rx);

            for i in 0..65536 {
                tx.start_send(i).unwrap();
            }
        })
    });
}

fn ring(c: &mut Criterion) {
    c.bench_function("ring", |b| {
        b.iter(|| async move {
            let (mut tx, rx) = spsc_rs::channel(65536);

            std::mem::forget(rx);

            for i in 0..65536 {
                tx.start_send(i).unwrap();
            }
        })
    });
}

criterion_group!(ring_bench, exact_ring, ring);
criterion_main!(ring_bench);

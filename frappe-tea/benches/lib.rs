use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rayon::prelude::*;

fn basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("linear iteration");

    group.bench_function("rayon", |b| {
        b.iter(|| {
            let items = black_box(vec![1u32; 10_000_000]);

            items.into_par_iter().sum::<u32>();
        });
    });
    group.bench_function("single-threaded", |b| {
        b.iter(|| {
            let items = black_box(vec![1u32; 10_000_000]);

            items.into_iter().sum::<u32>();
        });
    });

    group.finish();
}

criterion_group!(benches, basic);
criterion_main!(benches);
